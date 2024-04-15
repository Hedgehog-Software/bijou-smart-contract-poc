#![no_std]

mod constants;
mod oracle;
mod position;
mod position_data;
mod storage;
mod test;
mod token_data;
mod types;
mod user;

use core::cmp::{max, min};

use constants::{COLLATERAL_BUFFER, COLLATERAL_THRESHOLD, SCALE, TIME_TO_EXEC, TIME_TO_REPAY};
use oracle::get_oracle_spot_price;
use position::{create_position, get_used_positions_a, get_used_positions_b, set_position_valid};
use position_data::{
    are_positions_open, get_position_a, get_position_b, get_position_data, init_position_a,
    init_position_b, ocupy_one_position,
};
use soroban_sdk::{contract, contractimpl, token, Address, Env, Map, Symbol, Vec};
use storage::{
    get_admin, get_forward_rate, get_init_time, get_spot_rate, get_time_to_mature, put_admin,
    put_forward_rate, put_init_time, put_spot_rate, put_time_to_mature,
};
use token_data::{
    add_token_collateral_amount, add_token_deposited_amount, add_token_reclaimed_amount,
    add_token_returned_amount, add_token_swapped_amount, add_token_withdrawn_amount,
    add_token_withdrawn_collateral, get_token_a, get_token_a_address, get_token_b,
    get_token_b_address, init_token_a, init_token_b,
};
use types::{
    error::Error, position::Position, price_data::PriceData, stage::Stage, token::Token,
    user::User, user_liq_data::UserLiqData,
};
use user::{
    get_collateral, get_deposited_amount, get_deposited_token, get_reclaimed_amount,
    get_returned_amount, get_swapped_amount, get_user_balance, get_user_deposit,
    get_withdrawn_amount, get_withdrawn_collateral, has_not_repaid, is_liquidated, put_collateral,
    put_deposited_amount, put_deposited_token, put_is_liquidated, put_reclaimed_amount,
    put_returned_amount, put_swapped_amount, put_withdrawn_amount, put_withdrawn_collateral,
};

fn transfer(e: &Env, token: Address, to: Address, amount: i128) {
    token::Client::new(e, &token).transfer(&e.current_contract_address(), &to, &amount);
}

fn transfer_a(e: &Env, to: &Address, amount: i128) {
    transfer(e, get_token_a(e).address, to.clone(), amount);
}

fn transfer_b(e: &Env, to: &Address, amount: i128) {
    transfer(e, get_token_b(e).address, to.clone(), amount);
}

fn near_leg_time_reached(e: &Env) -> bool {
    let ledger_timestamp = e.ledger().timestamp();
    let init_time: u64 = get_init_time(&e);
    let exec_time: u64 = init_time + TIME_TO_EXEC;
    ledger_timestamp >= exec_time
}

fn set_spot_price(e: &Env) -> PriceData {
    let price_data = get_oracle_spot_price(&e);
    put_spot_rate(&e, price_data.price);
    price_data
}

fn has_near_leg_executed(e: &Env) -> bool {
    get_stage(&e) >= Stage::Swap
}

fn max_time_reached(e: &Env) -> bool {
    let ledger_timestamp = e.ledger().timestamp();
    let init_time: u64 = get_init_time(&e);
    let time_to_mature = get_time_to_mature(&e);
    let time_limit: u64 = init_time + TIME_TO_EXEC + time_to_mature + TIME_TO_REPAY;
    ledger_timestamp >= time_limit
}

fn get_min_collateral(e: &Env, to: &Address, spot_rate: i128, is_deposit_token_a: bool) -> i128 {
    let swapped_amount = get_swapped_amount(&e, &to);
    let og_spot_rate = get_spot_rate(&e);
    let forward_rate = get_forward_rate(&e);

    if is_deposit_token_a {
        let used_deposited_amount = convert_amount_token_b_to_a(swapped_amount, og_spot_rate);
        let to_return_amount = convert_amount_token_a_to_b(used_deposited_amount, forward_rate);
        let current_price = convert_amount_token_a_to_b(used_deposited_amount, spot_rate);
        let min_col = calculate_percentage(used_deposited_amount, COLLATERAL_BUFFER);
        if to_return_amount > current_price {
            let mtm = to_return_amount - current_price;
            let amount = convert_amount_token_b_to_a(mtm, spot_rate);
            max(calculate_percentage(amount, COLLATERAL_THRESHOLD), min_col)
        } else {
            min_col
        }
    } else {
        let used_deposited_amount = convert_amount_token_a_to_b(swapped_amount, og_spot_rate);
        let to_return_amount = convert_amount_token_b_to_a(used_deposited_amount, forward_rate);
        let current_price = convert_amount_token_b_to_a(used_deposited_amount, spot_rate);
        let min_col = calculate_percentage(used_deposited_amount, COLLATERAL_BUFFER);
        if to_return_amount > current_price {
            let mtm = to_return_amount - current_price;
            let amount = convert_amount_token_a_to_b(mtm, spot_rate);
            max(calculate_percentage(amount, COLLATERAL_THRESHOLD), min_col)
        } else {
            min_col
        }
    }
}

fn liquidate_user(e: &Env, to: &Address, from: &Address, spot_price: i128) -> i128 {
    let withdrawn_collateral = get_withdrawn_collateral(&e, &to);
    let collateral = get_collateral(&e, &to) - withdrawn_collateral;
    let mut reward_amount: i128 = 0;
    let expired_and_not_repaid = max_time_reached(&e) && has_not_repaid(&e, &to);

    if is_liquidated(&e, &to) {
        return 0;
    }

    if let Some(token) = get_deposited_token(&e, &to) {
        if token == get_token_a_address(&e) {
            // If user deposited a then it swapped b
            // User a needs to have 150 of the corresponding to token b in its collateral
            // we need to convert swapped amount into token a
            let min_collateral = get_min_collateral(&e, &to, spot_price, true);

            if (min_collateral > collateral) || expired_and_not_repaid {
                reward_amount = calculate_percentage(collateral, 1);
                put_is_liquidated(&e, &to, true);
                transfer_a(&e, &from, reward_amount);
            }
        } else {
            let min_collateral = get_min_collateral(&e, &to, spot_price, false);

            if (min_collateral > collateral) || expired_and_not_repaid {
                reward_amount = calculate_percentage(collateral, 1);
                put_is_liquidated(&e, &to, true);
                transfer_b(&e, &from, reward_amount);
            }
        }
    }
    reward_amount
}

fn get_stage(e: &Env) -> Stage {
    let ledger_timestamp = e.ledger().timestamp();
    let init_time = get_init_time(&e);
    let time_to_mature = get_time_to_mature(&e);
    let time_to_deposit = init_time + TIME_TO_EXEC;
    let time_to_swap = time_to_deposit + time_to_mature;
    let time_limit = time_to_swap + TIME_TO_REPAY;

    match ledger_timestamp {
        ts if ts < time_to_deposit => Stage::Deposit,
        ts if ts >= time_to_deposit && ts < time_to_swap => Stage::Swap,
        ts if ts >= time_to_swap && ts < time_limit => Stage::Repay,
        _ => Stage::Withdraw,
    }
}

// User
fn get_user_amount_to_repay(e: &Env, to: &Address) -> i128 {
    let forward_rate = get_forward_rate(&e);
    let spot_rate = get_spot_rate(&e);
    let swapped_amount = get_swapped_amount(&e, &to);
    let mut repay_amount: i128 = 0;
    if let Some(token) = get_deposited_token(&e, &to) {
        if token == get_token_a_address(&e) {
            let used_deposited_amount = convert_amount_token_b_to_a(swapped_amount, spot_rate);
            repay_amount = convert_amount_token_a_to_b(used_deposited_amount, forward_rate);
        } else {
            repay_amount = swapped_amount;
        }
    }
    repay_amount
}

fn is_authorized(e: &Env, to: &Address) -> bool {
    let admin_address = get_admin(&e).unwrap();
    to.clone() == admin_address
}

//Utils
fn convert_amount_token_a_to_b(amount: i128, rate: i128) -> i128 {
    amount * rate / SCALE
}

fn convert_amount_token_b_to_a(amount: i128, rate: i128) -> i128 {
    (amount * SCALE) / rate
}

fn calculate_percentage(amount: i128, rate: i128) -> i128 {
    (amount * rate) / 100
}

fn is_valid_token(e: &Env, token: Address) -> bool {
    let token_a_address = get_token_a_address(&e);
    let token_b_address = get_token_b_address(&e);
    token == token_a_address || token == token_b_address
}

fn calculate_used_deposited_amount(
    user: &Address,
    used_positions: Vec<Position>,
    total_other_deposited_amount: i128,
    base_deposit_amount: i128,
    base_converted_amount: i128,
    convert_currency: fn(i128, i128) -> i128,
    spot_rate: i128,
) -> i128 {
    let mut used_amount = 0;
    let mut acum = 0;

    for position in used_positions.iter() {
        if position.is_valid {
            acum += base_converted_amount;
            if position.address == user.clone() {
                used_amount += base_deposit_amount;
                if acum >= total_other_deposited_amount {
                    let surplus = convert_currency(acum - total_other_deposited_amount, spot_rate);
                    return max(used_amount - surplus, 0);
                }
            }
            if acum >= total_other_deposited_amount {
                return used_amount;
            }
        }
    }

    used_amount
}

fn get_used_deposited_amount(e: &Env, user: &Address) -> i128 {
    let token_a_data = get_token_a(&e);
    let token_b_data = get_token_b(&e);
    let position_a = get_position_a(&e);
    let position_b = get_position_b(&e);
    let spot_rate = get_spot_rate(&e);

    let amount = match token_a_data.address == get_deposited_token(&e, &user).unwrap() {
        true => {
            let used_positions_a = get_used_positions_a(&e);
            let total_other_deposited_amount = token_b_data.deposited_amount;
            let base_converted_amount =
                convert_amount_token_a_to_b(position_a.deposit_amount, spot_rate);
            calculate_used_deposited_amount(
                user,
                used_positions_a,
                total_other_deposited_amount,
                position_a.deposit_amount,
                base_converted_amount,
                convert_amount_token_b_to_a,
                spot_rate,
            )
        }
        false => {
            let used_positions_b = get_used_positions_b(&e);
            let total_other_deposited_amount = token_a_data.deposited_amount;
            let base_converted_amount =
                convert_amount_token_b_to_a(position_b.deposit_amount, spot_rate);
            calculate_used_deposited_amount(
                user,
                used_positions_b,
                total_other_deposited_amount,
                position_b.deposit_amount,
                base_converted_amount,
                convert_amount_token_a_to_b,
                spot_rate,
            )
        }
    };
    max(amount, 0)
}

fn calculate_amount_deposit_token_b(
    e: &Env,
    positions_token_a: u64,
    positions_token_b: u64,
    amount_deposit_token_a: i128,
) -> i128 {
    let spot_rate = get_spot_rate(&e);
    let total_amount_a: i128 = (positions_token_a as i128) * amount_deposit_token_a;
    let amount_deposit_amount_a: i128 = total_amount_a / (positions_token_b as i128);
    convert_amount_token_a_to_b(amount_deposit_amount_a, spot_rate)
}

fn get_users_liq_data(
    e: &Env,
    deposits: Vec<Position>,
    is_deposit_token_a: bool,
) -> Vec<UserLiqData> {
    let spot_rate = get_oracle_spot_price(&e).price;
    let mut unique_addresses: Map<Address, bool> = Map::new(&e);
    let mut users: Vec<UserLiqData> = Vec::new(&e);

    deposits.iter().for_each(|position| {
        unique_addresses.set(position.address, true);
    });

    unique_addresses.iter().for_each(|(address, _)| {
        users.push_back(UserLiqData {
            address: address.clone(),
            collateral: get_collateral(&e, &address),
            min_collateral: get_min_collateral(&e, &address, spot_rate, is_deposit_token_a),
            is_liquidated: is_liquidated(&e, &address),
        })
    });

    users
}

pub trait SwapTrait {
    // Initializes the contract.
    //
    // # Arguments
    //
    // * `admin` - Address of the admin,
    // * `token_a` - Address of token A to swap,
    // * `token_b` - Address of token B to swap,
    // * `name_token_a` - Symbol of token A to swap,
    // * `name_token_b` - Symbol of token B to swap,
    // * `forward_rate` - Forward rate,
    // * `duration` - Contract duration until the contract matures.
    // # Returns
    //
    // Spot rate or Error.
    fn initialize(
        e: Env,
        admin: Address,
        token_a: Address,
        token_b: Address,
        name_token_a: Symbol,
        name_token_b: Symbol,
        forward_rate: i128,
        duration: u64,
    ) -> Result<i128, Error>;

    // Set the positions' values.
    //
    // # Arguments
    //
    // * `from` - Address of the caller (Only admin can initialize the positions),
    // * `positions_token_a` - Quantity of positions for Token A,
    // * `positions_token_b` - Quantity of positions for Token B,
    // * `amount_deposit_token_a` - Amount to deposit in each position for Token A,
    // # Returns
    //
    // Amount to deposit in each position of Token B or Error.
    fn init_pos(
        e: Env,
        from: Address,
        positions_token_a: u64,
        positions_token_b: u64,
        amount_deposit_token_a: i128,
    ) -> Result<i128, Error>;

    // Deposit amount and collateral.
    // TODO: Add desired execution time
    //
    // # Arguments
    //
    // * `from` - Address of the user depositing,
    // * `token` - Address of the token to deposit,
    // * `amount` - Amount to deposit,
    // * `collateral` - Amount of collateral to deposit
    //
    // # Returns
    //
    // Tuple: total deposit amount and total collateral amount or Error.
    fn deposit(
        e: Env,
        from: Address,
        token: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(i128, i128), Error>;

    // Executes neag leg.
    //
    // # Returns
    //
    // Price and timestamp of spot rate or None if the asset is not supported.
    fn near_leg(e: Env) -> Result<PriceData, Error>;

    // Transfers the desired token
    // Can only be called in the Execution Stage.
    //
    // # Arguments
    //
    // * `from` - Address of the user executing the swap
    //
    // # Returns
    //
    // Swapped amount or Error if near leg was not executed.
    fn swap(e: Env, from: Address) -> Result<i128, Error>;

    // Liquidate the Address if can be liquidated
    // Only possible after neaN Leg is executed
    // Returns %1 of collateral if liquidated, 0 otherwise.
    //
    // # Arguments
    //
    // * `to` - Address of the user to liquidate,
    // * `from` - Address of the user executing the liquidation
    //
    // # Returns
    //
    // Reward amount if address liquidated, 0 if it was not or collateral was too low.
    fn liquidate(e: Env, to: Address, from: Address) -> i128;

    // Repays the amount previously swapped.
    //
    // # Arguments
    //
    // * `from` - Address of the user repaying,
    // * `token` - Address of the token to repay,
    // * `amount` - Amount to repay
    //
    // # Returns
    //
    // Tuple: (total repaid amount, amount to repay) or Error.
    fn repay(e: Env, from: Address, token: Address, amount: i128) -> Result<(i128, i128), Error>;

    // Withdraws the deposited amount using the forward rate.
    //
    // # Arguments
    //
    // * `from` - Address of the user withdrawing
    //
    // # Returns
    //
    // Tuple: (amount of token A withdrawn, amount of token B withdrawn) or Error.
    fn withdraw(e: Env, from: Address) -> Result<(i128, i128), Error>;

    // Transfers the initial deposit surplus.
    //
    // # Arguments
    //
    // * `from` - Address of the user reclaiming
    //
    // # Returns
    //
    // Reclaimed amount or an Error if the contract is open.
    fn reclaim(e: Env, from: Address) -> Result<i128, Error>;

    // Transfers the deposited collateral.
    //
    // # Arguments
    //
    // * `from` - Address of the user reclaiming
    //
    // # Returns
    //
    // Reclaimed collateral amount or Error if user was liquidated or contract is open.
    fn reclaim_col(e: Env, from: Address) -> Result<i128, Error>;

    // Returns a user's balance.
    //
    // # Arguments
    //
    // * `to` - Address of the user's balance
    //
    // # Returns
    //
    // User balance.
    fn balance(e: Env, to: Address) -> User;

    // Returns the spot rate.
    //
    // # Returns
    //
    // Spot rate value.
    fn spot_rate(e: Env) -> i128;

    // Returns the Admin address.
    //
    // # Returns
    //
    // Admin address or Panic if None.
    fn admin(e: Env) -> Address;

    // Returns the two tokens and its balances
    //
    // # Returns
    //
    // Tuple of Token Data.
    fn tokens(e: Env) -> (Token, Token);

    // Set the spot rate (Only for admin)
    //
    // # Arguments
    //
    // * `from` - Address of the user,
    // * `rate` - rate of spot rate
    //
    // # Returns
    //
    // None or Error.
    fn set_spot(e: Env, from: Address, rate: i128) -> Result<(), Error>;

    // Returns the current stage.
    //
    // # Returns
    //
    // Contract Stage.
    fn stage(e: Env) -> Stage;

    // Returns the deposits made in each token.
    //
    // # Returns
    //
    // Tuple containing arrays of deposits: (deposits for Token A, deposits for Token B).
    fn deposits(e: Env) -> (Vec<Position>, Vec<Position>);

    // Returns the users info for liquidation
    //
    // # Returns
    //
    // Tuple containing arrays of User Data: (Users for Token A, Users for Token B).
    fn users(e: Env) -> (Vec<UserLiqData>, Vec<UserLiqData>);

    // Transfer amount of token from contract to address
    fn transfer_admin(
        e: Env,
        from: Address,
        to: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error>;
}

#[contract]
struct Swap;

#[contractimpl]
impl SwapTrait for Swap {
    fn initialize(
        e: Env,
        admin: Address,
        token_a: Address,
        token_b: Address,
        name_token_a: Symbol,
        name_token_b: Symbol,
        forward_rate: i128,
        duration: u64,
    ) -> Result<i128, Error> {
        match get_admin(&e) {
            Some(_) => Err(Error::ContractAlreadyInitialized),
            None => {
                put_admin(&e, admin);
                init_token_a(&e, &token_a, name_token_a);
                init_token_b(&e, &token_b, name_token_b);
                put_forward_rate(&e, forward_rate);
                put_init_time(&e);
                put_time_to_mature(&e, duration);
                Ok(set_spot_price(&e).price)
            }
        }
    }

    fn init_pos(
        e: Env,
        from: Address,
        positions_token_a: u64,
        positions_token_b: u64,
        amount_deposit_token_a: i128,
    ) -> Result<i128, Error> {
        from.require_auth();

        if !is_authorized(&e, &from) {
            return Err(Error::Unauthorized);
        }

        init_position_a(&e, positions_token_a, amount_deposit_token_a);
        let amount_deposit_token_b = calculate_amount_deposit_token_b(
            &e,
            positions_token_a,
            positions_token_b,
            amount_deposit_token_a,
        );
        init_position_b(&e, positions_token_b, amount_deposit_token_b);

        Ok(amount_deposit_token_b)
    }

    fn deposit(
        e: Env,
        from: Address,
        token: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(i128, i128), Error> {
        from.require_auth();

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
        }

        let near_leg_executed = has_near_leg_executed(&e);
        let position_data = get_position_data(&e, &token);
        let min_collateral = calculate_percentage(amount, COLLATERAL_BUFFER);

        if collateral < min_collateral {
            return Err(Error::InsufficientCollateral);
        }

        if !near_leg_executed && !are_positions_open(&position_data) {
            return Err(Error::AllPositionsAreUsed);
        }

        if !near_leg_executed && amount != 0 && amount != position_data.deposit_amount {
            return Err(Error::DepositAmountDoesntMatchPosition);
        }

        if near_leg_executed && amount != 0 {
            return Err(Error::CollateralOnlyCanBeDeposited);
        }

        match get_deposited_token(&e, &from) {
            Some(p) => {
                if p != token {
                    return Err(Error::DifferentDepositedToken);
                }
            }
            None => put_deposited_token(&e, &from, &token),
        }

        if !near_leg_executed && amount > 0 {
            let position_index = create_position(&e, &from, &token);

            token::Client::new(&e, &token).transfer(&from, &e.current_contract_address(), &amount);
            put_deposited_amount(&e, &from, amount);
            add_token_deposited_amount(&e, &token, amount);

            set_position_valid(&e, position_index, &token);
            ocupy_one_position(&e, &token, &position_data);
        }

        if collateral > 0 {
            token::Client::new(&e, &token).transfer(
                &from,
                &e.current_contract_address(),
                &collateral,
            );
            put_collateral(&e, &from, collateral);
            add_token_collateral_amount(&e, &token, collateral);
        }

        Ok(get_user_deposit(&e, &from))
    }

    fn swap(e: Env, from: Address) -> Result<i128, Error> {
        from.require_auth();

        if get_stage(&e) != Stage::Swap {
            return Err(Error::WrongStageToSwap);
        }

        let mut swap_amount: i128 = 0;
        let spot_rate: i128 = get_spot_rate(&e);

        if let Some(token) = get_deposited_token(&e, &from) {
            if token == get_token_a_address(&e) {
                let used_deposited_amount = get_used_deposited_amount(&e, &from);
                let exp_swap_amount = convert_amount_token_a_to_b(used_deposited_amount, spot_rate);

                let token_b_data = get_token_b(&e);
                let token_b_available_amount =
                    token_b_data.deposited_amount - token_b_data.swapped_amount;

                swap_amount = min(exp_swap_amount, token_b_available_amount);
                transfer_b(&e, &from, swap_amount);
                put_swapped_amount(&e, &from, swap_amount);
                add_token_swapped_amount(&e, &token_b_data.address, swap_amount);
            } else {
                let used_deposited_amount = get_used_deposited_amount(&e, &from);
                let exp_swap_amount = convert_amount_token_b_to_a(used_deposited_amount, spot_rate);

                let token_a_data = get_token_a(&e);
                let token_a_available_amount =
                    token_a_data.deposited_amount - token_a_data.swapped_amount;

                swap_amount = min(exp_swap_amount, token_a_available_amount);
                transfer_a(&e, &from, swap_amount);
                put_swapped_amount(&e, &from, swap_amount);
                add_token_swapped_amount(&e, &token_a_data.address, swap_amount);
            }
        }

        Ok(swap_amount)
    }

    fn reclaim(e: Env, from: Address) -> Result<i128, Error> {
        from.require_auth();

        if get_stage(&e) == Stage::Deposit {
            return Err(Error::TimeNotReached);
        }

        let deposited_amount = get_deposited_amount(&e, &from);
        let used_deposited_amount = get_used_deposited_amount(&e, &from);
        let reclaimed_amount = get_reclaimed_amount(&e, &from);
        let amount = deposited_amount - used_deposited_amount - reclaimed_amount;

        if amount <= 9 {
            return Ok(0);
        }

        if let Some(token) = get_deposited_token(&e, &from) {
            if token == get_token_a_address(&e) {
                transfer_a(&e, &from, amount);
                add_token_reclaimed_amount(&e, &token, amount);
                put_reclaimed_amount(&e, &from, amount);
            } else {
                transfer_b(&e, &from, amount);
                add_token_reclaimed_amount(&e, &token, amount);
                put_reclaimed_amount(&e, &from, amount);
            }
        }

        Ok(amount)
    }

    fn reclaim_col(e: Env, from: Address) -> Result<i128, Error> {
        from.require_auth();

        if get_stage(&e) == Stage::Deposit {
            return Err(Error::TimeNotReached);
        }

        let used_deposited_amount = get_used_deposited_amount(&e, &from);
        let token_a_address = get_token_a_address(&e);
        let min_col =
            if used_deposited_amount != 0 && (is_liquidated(&e, &from) || !max_time_reached(&e)) {
                let spot_rate = get_oracle_spot_price(&e).price;
                let deposited_token = get_deposited_token(&e, &from).unwrap();
                get_min_collateral(&e, &from, spot_rate, token_a_address == deposited_token)
            } else {
                0
            };

        let collateral_amount = get_collateral(&e, &from);
        let withdrawn_collateral_amount = get_withdrawn_collateral(&e, &from);
        let withdraw_amount = collateral_amount - min_col - withdrawn_collateral_amount;

        if withdraw_amount <= 0 {
            return Ok(0);
        }

        if let Some(token) = get_deposited_token(&e, &from) {
            if token == token_a_address {
                transfer_a(&e, &from, withdraw_amount);
                put_withdrawn_collateral(&e, &from, withdraw_amount);
                add_token_withdrawn_collateral(&e, &token, withdraw_amount);
            } else {
                transfer_b(&e, &from, withdraw_amount);
                put_withdrawn_collateral(&e, &from, withdraw_amount);
                add_token_withdrawn_collateral(&e, &token, withdraw_amount);
            }
        }

        Ok(withdraw_amount)
    }

    fn balance(e: Env, to: Address) -> User {
        to.require_auth();
        get_user_balance(&e, &to)
    }

    fn liquidate(e: Env, to: Address, from: Address) -> i128 {
        from.require_auth();

        let spot_price: i128 = get_oracle_spot_price(&e).price;
        liquidate_user(&e, &to, &from, spot_price)
    }

    fn repay(e: Env, from: Address, token: Address, amount: i128) -> Result<(i128, i128), Error> {
        from.require_auth();

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
        }

        if is_liquidated(&e, &from) {
            return Err(Error::LiquidatedUser);
        }

        let deposited_token = get_deposited_token(&e, &from).unwrap();
        let token_a_address = get_token_a_address(&e);
        let token_b_address = get_token_b_address(&e);
        if deposited_token == token_a_address {
            if token != token_b_address {
                return Err(Error::WrongRepayToken);
            }
        } else {
            if token != token_a_address {
                return Err(Error::WrongRepayToken);
            }
        }

        let prev_total_amount_to_repay = get_user_amount_to_repay(&e, &from);
        let prev_total_returned_amount = get_returned_amount(&e, &from);
        let repay_amount = min(
            amount,
            prev_total_amount_to_repay - prev_total_returned_amount,
        );

        if repay_amount <= 0 {
            return Err(Error::AlreadyRepaid);
        }

        token::Client::new(&e, &token).transfer(
            &from,
            &e.current_contract_address(),
            &repay_amount,
        );
        put_returned_amount(&e, &from, repay_amount);
        add_token_returned_amount(&e, &token, repay_amount);

        let total_returned_amount = get_returned_amount(&e, &from);
        let total_amount_to_repay = get_user_amount_to_repay(&e, &from);
        Ok((total_returned_amount, total_amount_to_repay))
    }

    fn withdraw(e: Env, from: Address) -> Result<(i128, i128), Error> {
        from.require_auth();

        let forward_rate = get_forward_rate(&e);
        let returned_amount = get_returned_amount(&e, &from);
        let withdrawn_amount = get_withdrawn_amount(&e, &from);
        let deposited_token = get_deposited_token(&e, &from).unwrap();
        let token_a_data = get_token_a(&e);
        let token_b_data = get_token_b(&e);
        let mut withdraw_amount_a: i128 = 0;
        let mut withdraw_amount_b: i128 = 0;

        if !max_time_reached(&e) {
            return Err(Error::TimeNotReached);
        }

        if is_liquidated(&e, &from) {
            return Err(Error::LiquidatedUser);
        }

        if deposited_token == token_a_data.address {
            let token_a_available_amount =
                token_a_data.returned_amount - token_a_data.withdrawn_amount;
            let converted_returned_amount =
                convert_amount_token_b_to_a(returned_amount, forward_rate);
            let exp_withdraw = converted_returned_amount - withdrawn_amount;
            withdraw_amount_a = min(exp_withdraw, token_a_available_amount);

            if withdraw_amount_a > 0 {
                transfer_a(&e, &from, withdraw_amount_a);
                add_token_withdrawn_amount(&e, &deposited_token, withdraw_amount_a);
                put_withdrawn_amount(&e, &from, withdraw_amount_a);
            }

            if exp_withdraw > withdraw_amount_a {
                //    return token b to compensate
                let token_b_address = token_b_data.address;
                let rem_withdraw = exp_withdraw - withdraw_amount_a;
                let exp_withdraw_amount_b = convert_amount_token_a_to_b(rem_withdraw, forward_rate);
                let used_returned = convert_amount_token_a_to_b(withdraw_amount_a, forward_rate);
                let use_from_returned = min(returned_amount - used_returned, exp_withdraw_amount_b);
                let max_collateral_available =
                    calculate_percentage(use_from_returned, COLLATERAL_BUFFER);
                let use_from_col = min(
                    exp_withdraw_amount_b - use_from_returned,
                    max_collateral_available,
                );
                withdraw_amount_b = use_from_returned + use_from_col;
                let converted_withdraw_amount_b =
                    convert_amount_token_b_to_a(withdraw_amount_b, forward_rate);

                transfer_b(&e, &from, withdraw_amount_b);
                put_withdrawn_amount(&e, &from, converted_withdraw_amount_b);
                add_token_withdrawn_amount(&e, &token_b_address, use_from_returned);
                add_token_withdrawn_collateral(&e, &token_b_address, use_from_col);
            }
        } else {
            // User returned token_a
            let token_b_available_amount =
                token_b_data.returned_amount - token_b_data.withdrawn_amount;
            let converted_returned_amount =
                convert_amount_token_a_to_b(returned_amount, forward_rate);
            let exp_withdraw = converted_returned_amount - withdrawn_amount;
            withdraw_amount_b = min(exp_withdraw, token_b_available_amount);

            if withdraw_amount_b > 0 {
                transfer_b(&e, &from, withdraw_amount_b);
                add_token_withdrawn_amount(&e, &deposited_token, withdraw_amount_b);
                put_withdrawn_amount(&e, &from, withdraw_amount_b);
            }

            if exp_withdraw > withdraw_amount_b {
                let rem_withdraw = exp_withdraw - withdraw_amount_b;
                let exp_withdraw_amount_a = convert_amount_token_b_to_a(rem_withdraw, forward_rate);
                let used_returned = convert_amount_token_b_to_a(withdraw_amount_b, forward_rate);
                let use_from_returned = min(returned_amount - used_returned, exp_withdraw_amount_a);
                let max_collateral_available =
                    calculate_percentage(use_from_returned, COLLATERAL_BUFFER);
                let use_from_col = min(
                    exp_withdraw_amount_a - use_from_returned,
                    max_collateral_available,
                );
                withdraw_amount_a = use_from_returned + use_from_col;
                let converted_withdraw_amount_a =
                    convert_amount_token_a_to_b(withdraw_amount_a, forward_rate);

                transfer_a(&e, &from, withdraw_amount_a);
                put_withdrawn_amount(&e, &from, converted_withdraw_amount_a);
                add_token_withdrawn_amount(&e, &token_a_data.address, use_from_returned);
                add_token_withdrawn_collateral(&e, &token_a_data.address, use_from_col);
            }
        }

        Ok((withdraw_amount_a, withdraw_amount_b))
    }

    fn spot_rate(e: Env) -> i128 {
        get_spot_rate(&e)
    }

    fn admin(e: Env) -> Address {
        get_admin(&e).unwrap()
    }

    fn near_leg(e: Env) -> Result<PriceData, Error> {
        if !near_leg_time_reached(&e) {
            return Err(Error::TimeNotReached);
        }

        if get_spot_rate(&e) != 0 {
            return Err(Error::SpotRateAlreadyDefined);
        }

        Ok(set_spot_price(&e))
    }

    fn tokens(e: Env) -> (Token, Token) {
        let token_a = get_token_a(&e);
        let token_b = get_token_b(&e);
        (token_a, token_b)
    }

    fn set_spot(e: Env, from: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();

        if !is_authorized(&e, &from) {
            return Err(Error::Unauthorized);
        }

        Ok(put_spot_rate(&e, amount))
    }

    fn stage(e: Env) -> Stage {
        get_stage(&e)
    }

    fn deposits(e: Env) -> (Vec<Position>, Vec<Position>) {
        (get_used_positions_a(&e), get_used_positions_b(&e))
    }

    fn users(e: Env) -> (Vec<UserLiqData>, Vec<UserLiqData>) {
        let deposits_a = get_used_positions_a(&e);
        let deposits_b = get_used_positions_b(&e);
        (
            get_users_liq_data(&e, deposits_a, true),
            get_users_liq_data(&e, deposits_b, false),
        )
    }

    fn transfer_admin(
        e: Env,
        from: Address,
        to: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        from.require_auth();

        if !is_authorized(&e, &from) {
            return Err(Error::Unauthorized);
        }

        if token == get_token_a_address(&e) {
            Ok(transfer_a(&e, &to, amount))
        } else if token == get_token_b_address(&e) {
            Ok(transfer_b(&e, &to, amount))
        } else {
            Err(Error::InvalidToken)
        }
    }
}
