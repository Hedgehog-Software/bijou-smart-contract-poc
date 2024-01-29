#![no_std]

mod constants;
mod position;
mod position_data;
mod storage_types;
mod test;
mod token_data;
mod types;
mod user;

use core::cmp::min;

use constants::{COLLATERAL_BUFFER, ORACLE_ADDRESS, SCALE, TIME_TO_EXEC, TIME_TO_REPAY};
use position::{create_position, get_used_positions_a, get_used_positions_b, set_position_valid};
use position_data::{
    are_positions_open, get_position_a, get_position_b, get_position_data, init_position_a,
    init_position_b, ocupy_one_position,
};
use soroban_sdk::{contract, contractimpl, token, vec, Address, Env, String, Symbol, Vec};
use storage_types::DataKey;
use token_data::{
    add_token_collateral_amount, add_token_deposited_amount, add_token_returned_amount,
    add_token_swapped_amount, add_token_withdrawn_amount, get_token_a, get_token_a_address,
    get_token_b, get_token_b_address, init_token_a, init_token_b,
};
use types::{
    asset::Asset, error::Error, position::Position, price_data::PriceData, state::State,
    token::Token, user::User,
};
use user::{
    get_collateral, get_deposited_amount, get_deposited_token, get_returned_amount,
    get_swapped_amount, get_user_balance, get_user_deposit, get_withdrawn_collateral,
    has_not_repaid, is_liquidated, put_collateral, put_deposited_amount, put_deposited_token,
    put_returned_amount, put_swapped_amount, put_withdrawn_amount, put_withdrawn_collateral,
};

fn get_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&DataKey::Admin)
}

fn get_spot_rate(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::SpotRate)
        .unwrap_or_default()
}

fn get_forward_rate(e: &Env) -> i128 {
    e.storage().persistent().get(&DataKey::ForwardRate).unwrap()
}

fn get_init_time(e: &Env) -> u64 {
    e.storage().persistent().get(&DataKey::InitTime).unwrap()
}

fn get_time_to_mature(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&DataKey::TimeToMature)
        .unwrap()
}

fn put_admin(e: &Env, address: Address) {
    e.storage().persistent().set(&DataKey::Admin, &address);
}

fn put_forward_rate(e: &Env, rate: i128) {
    e.storage().persistent().set(&DataKey::ForwardRate, &rate);
}

fn put_spot_rate(e: &Env, amount: i128) {
    e.storage().persistent().set(&DataKey::SpotRate, &amount);
}

fn put_init_time(e: &Env) {
    let time = e.ledger().timestamp();
    e.storage().persistent().set(&DataKey::InitTime, &time);
}

fn put_time_to_mature(e: &Env, duration: u64) {
    e.storage()
        .persistent()
        .set(&DataKey::TimeToMature, &duration);
}

fn put_is_liquidated(e: &Env, to: &Address, val: bool) {
    let key = DataKey::ReturnedAmount(to.clone());
    e.storage().persistent().set(&key, &val);
}

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

fn exec_near_leg(e: &Env) -> PriceData {
    let price_data = get_oracle_spot_price(&e);
    put_spot_rate(&e, price_data.price);
    price_data
}

fn has_near_leg_executed(e: &Env) -> bool {
    get_spot_rate(&e) != 0
}

fn max_time_reached(e: &Env) -> bool {
    let ledger_timestamp = e.ledger().timestamp();
    let init_time: u64 = get_init_time(&e);
    let time_to_mature = get_time_to_mature(&e);
    let time_limit: u64 = init_time + TIME_TO_EXEC + time_to_mature + TIME_TO_REPAY;
    ledger_timestamp >= time_limit
}

fn liquidate_user(e: &Env, to: &Address, from: &Address, spot_price: i128) -> i128 {
    let forward_rate = get_forward_rate(&e);
    let collateral = get_collateral(&e, &to);
    let swapped_amount = get_swapped_amount(&e, &to);
    let mut reward_amount: i128 = 0;

    let expired_and_not_repaid = max_time_reached(&e) && has_not_repaid(&e, &to);

    if let Some(token) = get_deposited_token(&e, &to) {
        if token == get_token_a_address(&e) {
            // If user deposited a then it swapped b
            // User a needs to have 150 of the corresponding to token b in its collateral
            // we need to convert swapped amount into token a
            let min_collateral = convert_amount_token_b_to_a(
                (COLLATERAL_BUFFER * swapped_amount) / 100,
                forward_rate,
            );
            let curr_collateral = convert_amount_token_b_to_a(collateral, spot_price);

            if (min_collateral > curr_collateral) || expired_and_not_repaid {
                reward_amount = collateral / 100;
                put_is_liquidated(&e, &to, true);
                transfer_a(&e, &from, reward_amount);
            }
        } else {
            let min_collateral = convert_amount_token_a_to_b(
                (COLLATERAL_BUFFER * swapped_amount) / 100,
                forward_rate,
            );
            let curr_collateral = convert_amount_token_a_to_b(collateral, spot_price);

            if (min_collateral > curr_collateral) || expired_and_not_repaid {
                reward_amount = collateral / 100;
                put_is_liquidated(&e, &to, true);
                transfer_b(&e, &from, reward_amount);
            }
        }
    }
    reward_amount
}

fn get_state(e: &Env) -> State {
    let ledger_timestamp = e.ledger().timestamp();
    let init_time: u64 = get_init_time(&e);
    let time_to_mature = get_time_to_mature(&e);
    let spot_rate = get_spot_rate(&e);
    let time_to_deposit = init_time + TIME_TO_EXEC;
    let time_to_swap = time_to_deposit + time_to_mature;
    let time_limit = time_to_swap + TIME_TO_REPAY;

    match ledger_timestamp {
        ts if ts < time_to_deposit || spot_rate == 0 => State::Deposit,
        ts if ts >= time_to_deposit && ts < time_to_swap => State::Swap,
        ts if ts >= time_to_swap && ts < time_limit => State::Repay,
        _ => State::Withdraw,
    }
}

// Oracle
fn get_oracle_spot_price(e: &Env) -> PriceData {
    // return PriceData {
    //     price: 100_000_000_000_000,
    //     timestamp: 2,
    // };

    let oracle_address: String = String::from_str(&e, ORACLE_ADDRESS);
    let target: Address = Address::from_string(&oracle_address);
    let func: Symbol = Symbol::new(&e, "x_last_price");
    let base_token = get_token_a(&e).name;
    let base_asset = Asset::Other(base_token);
    let quote_token = get_token_b(&e).name;
    let quote_asset = Asset::Other(quote_token);
    let args = vec![&e, base_asset, quote_asset].to_vals();
    e.invoke_contract::<PriceData>(&target, &func, args)
}

// User

fn get_user_amount_to_repay(e: &Env, to: &Address) -> i128 {
    let forward_rate = get_forward_rate(&e);
    let spot_rate = get_spot_rate(&e);
    let swapped_amount = get_swapped_amount(&e, &to);
    let mut repay_amount: i128 = 0;
    if let Some(token) = get_deposited_token(&e, &to) {
        let token_a_address = get_token_a_address(&e);
        if token == token_a_address {
            repay_amount = swapped_amount;
        } else {
            let used_deposited_amount = convert_amount_token_a_to_b(swapped_amount, spot_rate);
            repay_amount = convert_amount_token_b_to_a(used_deposited_amount, forward_rate);
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

fn is_valid_token(e: &Env, token: Address) -> bool {
    let token_a_address = get_token_a_address(&e);
    let token_b_address = get_token_b_address(&e);
    token == token_a_address || token == token_b_address
}

// Deposit
// fn handle_amount_deposit(e: &Env, to: &Address, token: &Address, amount: i128) {
//     if !near_leg_executed && amount != 0 {
//         let position_index = create_position(&e, &to, &token);

//         token::Client::new(&e, &token).transfer(&to, &e.current_contract_address(), &amount);
//         put_deposited_amount(&e, &to, amount);
//         add_token_deposited_amount(&e, &token, amount);
//         ocupy_one_position(&e, &token, &position_data);

//         set_position_valid(&e, position_index, &token);
//     }
// }

// Amount that can swap
//

fn calculate_used_deposited_amount(
    user: &Address,
    used_positions: Vec<Position>,
    total_other_deposited_amount: i128,
    base_deposit_amount: i128,
    base_converted_amount: i128,
) -> i128 {
    let mut used_amount: i128 = 0;
    let mut acum = 0;

    for position in used_positions.iter() {
        if position.is_valid {
            acum += base_converted_amount;
            if position.address == user.clone() {
                used_amount += base_deposit_amount;
            }
            if acum >= total_other_deposited_amount {
                return used_amount;
            }
        }
    }

    used_amount
}

pub fn get_used_deposited_amount(e: &Env, user: &Address) -> i128 {
    let token_a_data = get_token_a(&e);
    let token_b_data = get_token_b(&e);
    let position_a = get_position_a(&e);
    let position_b = get_position_b(&e);
    let spot_rate = get_spot_rate(&e);

    match token_a_data.address == get_deposited_token(&e, &user).unwrap() {
        true => {
            let used_positions_a = get_used_positions_a(&e);
            let total_other_deposited_amount = token_b_data.deposited_amount;
            let base_converted_amount =
                convert_amount_token_a_to_b(position_a.deposit_amount, spot_rate);
            return calculate_used_deposited_amount(
                user,
                used_positions_a,
                total_other_deposited_amount,
                position_a.deposit_amount,
                base_converted_amount,
            );
        }
        false => {
            let used_positions_b = get_used_positions_b(&e);
            let total_other_deposited_amount = token_a_data.deposited_amount;
            let base_converted_amount =
                convert_amount_token_b_to_a(position_b.deposit_amount, spot_rate);
            return calculate_used_deposited_amount(
                user,
                used_positions_b,
                total_other_deposited_amount,
                position_b.deposit_amount,
                base_converted_amount,
            );
        }
    }
}

pub trait SwapTrait {
    // Sets the token contract addresses for this pooli128
    //
    // # Arguments
    //
    // * `admin` - Address of admin
    // * `token_a` - Address of token A to swap
    // * `token_b` - Address of token B to swap
    // * `name_token_a` - Symbol of token A to swap,
    // * `name_token_b` - Symbol of token B to swap,
    // * `forward_rate` - forward rate
    // * `duration` - Contract duration until the contract matures
    // # Returns
    //
    // None or Error
    fn initialize(
        e: Env,
        admin: Address,
        token_a: Address,
        token_b: Address,
        name_token_a: Symbol,
        name_token_b: Symbol,
        forward_rate: i128,
        duration: u64,
    ) -> Result<(), Error>;

    // Set the positions values
    //
    // # Arguments
    //
    // * `from` - Address of caller (Only admin can initialize the positions),
    // * `positions_token_a` - Quantity of positions of token A,
    // * `positions_token_b` - Quantity of positions of token B,
    // * `amount_deposit_token_a` - Amount to deposit in each position of token A,
    // * `amount_deposit_token_b` - Amount to deposit in each position of token B,
    // # Returns
    //
    // None or Error
    fn init_pos(
        e: Env,
        from: Address,
        positions_token_a: u64,
        positions_token_b: u64,
        amount_deposit_token_a: i128,
        amount_deposit_token_b: i128,
    ) -> Result<(), Error>;

    // Deposits to: User, token: Address of token to deposit amount
    // TODO: Add desired execution time
    //
    // # Arguments
    //
    // * `to` - Address of user depositing
    // * `token` - Address of token to deposit
    // * `amount` - Amount to deposit
    // * `collateral` - Amount of collateral to deposit
    //
    // # Returns
    //
    // Tuple total deposit amout and total collateral amount or Error
    fn deposit(
        e: Env,
        to: Address,
        token: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(i128, i128), Error>;

    // Executes neag leg
    //
    // # Returns
    //
    // Price and its timestamp of spot rate or None if the asset is not supported
    fn near_leg(e: Env) -> Result<PriceData, Error>;

    // Transfers the desired token
    // Can only be called in the Execution State
    //
    // # Arguments
    //
    // * `to` - Address of user executing swap
    //
    // # Returns
    //
    // Swapped amount or Error if near leg was not executed
    fn swap(e: Env, to: Address) -> Result<i128, Error>;

    // Liquidate the Address if can be liquidated
    // Only possible in the Execution and Completion State
    // Returns %1 of collateral if liquidated, 0 otherwise
    //
    // # Arguments
    //
    // * `to` - Address of user to liquidate
    // * `from` - Address of user executing liquidation
    //
    // # Returns
    //
    // Reward amount if address liquidated, 0 if it was not or collateral was too low
    fn liquidate(e: Env, to: Address, from: Address) -> i128;

    fn liq_adm(e: Env, to: Address, from: Address, spot_price: i128) -> Result<i128, Error>;

    // To repay the amount previously swapped
    //
    // # Arguments
    //
    // * `to` - Address of user repaying
    // * `token` - Address of token to repay
    // * `amount` - Amount to repay
    //
    // # Returns
    //
    // Tuple (total repaid amount, amount to repay) or Error
    fn repay(e: Env, to: Address, token: Address, amount: i128) -> Result<(i128, i128), Error>;

    // Withdraw the swapped amount (using forward rate)
    //
    // # Arguments
    //
    // * `to` - Address of user withdrawing
    //
    // # Returns
    //
    // Withdrawn amount or Error
    fn withdraw(e: Env, to: Address) -> Result<i128, Error>;

    // Transfers the initial deposit surplus
    //
    // # Arguments
    //
    // * `to` - Address of user reclaiming
    //
    // # Returns
    //
    // Returned amount or Error if contract is open
    fn reclaim(e: Env, to: Address) -> Result<i128, Error>;

    // Transers the deposit collateral
    //
    // # Arguments
    //
    // * `to` - Address of user reclaiming
    //
    // # Returns
    //
    // Returned amount or Error if user was liquidated or contract is open
    fn reclaim_col(e: Env, to: Address) -> Result<i128, Error>;

    // Returns a users balance
    //
    // # Arguments
    //
    // * `to` - Address of user balance
    //
    // # Returns
    //
    // User balance
    fn balance(e: Env, to: Address) -> User;

    // Returns the spot rate
    //
    // # Returns
    //
    // Spot rate value
    fn spot_rate(e: Env) -> i128;

    // Returns the Admin address
    //
    // # Returns
    //
    // Admin address or Panic if None
    fn admin(e: Env) -> Address;

    // Returns the two tokens and its balances
    //
    // # Returns
    //
    // Tuple of Token Data
    fn tokens(e: Env) -> (Token, Token);

    // Set the spot rate (Only for admin)
    //
    // # Arguments
    //
    // * `to` - Address of user
    // * `rate` - rate of spot rate
    //
    // # Returns
    //
    // None    // * `to` - Address of user
    fn set_spot(e: Env, to: Address, rate: i128) -> Result<(), Error>;

    // Returns the current swap state
    //
    // # Returns
    //
    // Contract State
    fn state(e: Env) -> State;

    fn deposits(e: Env) -> (Vec<Position>, Vec<Position>);
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
    ) -> Result<(), Error> {
        match get_admin(&e) {
            Some(_) => Err(Error::ContractAlreadyInitialized),
            None => {
                put_admin(&e, admin);
                init_token_a(&e, &token_a, name_token_a);
                init_token_b(&e, &token_b, name_token_b);
                put_forward_rate(&e, forward_rate);
                put_init_time(&e);
                put_time_to_mature(&e, duration);
                Ok(())
            }
        }
    }

    fn init_pos(
        e: Env,
        from: Address,
        positions_token_a: u64,
        positions_token_b: u64,
        amount_deposit_token_a: i128,
        amount_deposit_token_b: i128,
    ) -> Result<(), Error> {
        if !is_authorized(&e, &from) {
            return Err(Error::Unauthorized);
        }
        init_position_a(&e, positions_token_a, amount_deposit_token_a);
        init_position_b(&e, positions_token_b, amount_deposit_token_b);
        Ok(())
    }

    fn deposit(
        e: Env,
        to: Address,
        token: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(i128, i128), Error> {
        to.require_auth();

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
        }

        let near_leg_executed = has_near_leg_executed(&e);
        let position_data = get_position_data(&e, &token);

        if !near_leg_executed && !are_positions_open(&position_data) {
            return Err(Error::AllPositionsAreUsed);
        }

        if !near_leg_executed && amount != position_data.deposit_amount {
            return Err(Error::DepositAmountDoesntMatchPosition);
        }

        if near_leg_executed && amount != 0 {
            return Err(Error::CollateralOnlyCanBeDeposited);
        }

        match get_deposited_token(&e, &to) {
            Some(p) => {
                if p != token {
                    return Err(Error::DifferentDepositedToken);
                }
            }
            None => put_deposited_token(&e, &to, &token),
        }

        if !near_leg_executed && amount != 0 {
            let position_index = create_position(&e, &to, &token);

            token::Client::new(&e, &token).transfer(&to, &e.current_contract_address(), &amount);
            put_deposited_amount(&e, &to, amount);
            add_token_deposited_amount(&e, &token, amount);

            set_position_valid(&e, position_index, &token);
            ocupy_one_position(&e, &token, &position_data);
        }

        if collateral != 0 {
            token::Client::new(&e, &token).transfer(
                &to,
                &e.current_contract_address(),
                &collateral,
            );
            put_collateral(&e, &to, collateral);
            add_token_collateral_amount(&e, &token, collateral);
        }

        Ok(get_user_deposit(&e, &to))
    }

    fn swap(e: Env, to: Address) -> Result<i128, Error> {
        to.require_auth();

        if get_spot_rate(&e) == 0 {
            return Err(Error::NearLegNotExecuted);
        }

        let mut swap_amount: i128 = 0;
        let spot_rate = get_spot_rate(&e);

        if let Some(token) = get_deposited_token(&e, &to) {
            if token == get_token_a_address(&e) {
                let used_deposited_amount = get_used_deposited_amount(&e, &to);
                let exp_swap_amount = convert_amount_token_a_to_b(used_deposited_amount, spot_rate);

                let token_b_data = get_token_b(&e);
                let token_b_available_amount =
                    token_b_data.deposited_amount - token_b_data.swapped_amount;

                swap_amount = min(exp_swap_amount, token_b_available_amount);
                put_swapped_amount(&e, &to, swap_amount);
                add_token_swapped_amount(&e, &token_b_data.address, swap_amount);
                transfer_b(&e, &to, swap_amount);
            } else {
                let used_deposited_amount = get_used_deposited_amount(&e, &to);
                let exp_swap_amount = convert_amount_token_b_to_a(used_deposited_amount, spot_rate);

                let token_a_data = get_token_a(&e);
                let token_a_available_amount =
                    token_a_data.deposited_amount - token_a_data.swapped_amount;

                swap_amount = min(exp_swap_amount, token_a_available_amount);
                put_swapped_amount(&e, &to, swap_amount);
                add_token_swapped_amount(&e, &token_a_data.address, swap_amount);
                transfer_a(&e, &to, swap_amount);
            }
        }

        Ok(swap_amount)
    }

    fn reclaim(e: Env, to: Address) -> Result<i128, Error> {
        to.require_auth();

        // TODO: Make sure the contract was already executed
        if !max_time_reached(&e) {
            return Err(Error::ContractStillOpen);
        }

        let mut amount: i128 = 0;

        let spot_rate = get_spot_rate(&e);
        if let Some(token) = get_deposited_token(&e, &to) {
            if token == get_token_a_address(&e) {
                let user_deposited_amount = get_deposited_amount(&e, &to);
                let user_swapped_amount = get_swapped_amount(&e, &to);
                let amount_token_b_to_a =
                    convert_amount_token_b_to_a(user_swapped_amount, spot_rate);

                if user_deposited_amount > amount_token_b_to_a {
                    amount = user_deposited_amount - amount_token_b_to_a;
                    add_token_returned_amount(&e, &token, amount);
                    put_returned_amount(&e, &to, amount);
                    transfer_a(&e, &to, amount);
                }
            } else {
                let user_deposited_amount = get_deposited_amount(&e, &to);
                let user_swapped_amount = get_swapped_amount(&e, &to);
                let amount_token_a_to_b =
                    convert_amount_token_a_to_b(user_swapped_amount, spot_rate);

                if user_deposited_amount > amount_token_a_to_b {
                    amount = user_deposited_amount - amount_token_a_to_b;
                    add_token_returned_amount(&e, &token, amount);
                    put_returned_amount(&e, &to, amount);
                    transfer_b(&e, &to, amount);
                }
            }
        }

        Ok(amount)
    }

    fn reclaim_col(e: Env, to: Address) -> Result<i128, Error> {
        to.require_auth();

        if is_liquidated(&e, &to) {
            return Err(Error::LiquidatedUser);
        }

        if !max_time_reached(&e) {
            return Err(Error::ContractStillOpen);
        }

        let collateral_amount = get_collateral(&e, &to);
        let withdrawn_collateral_amount = get_withdrawn_collateral(&e, &to);
        let withdraw_amount = collateral_amount - withdrawn_collateral_amount;

        if withdraw_amount <= 0 {
            return Ok(0);
        }

        if let Some(token) = get_deposited_token(&e, &to) {
            if token == get_token_a_address(&e) {
                put_withdrawn_collateral(&e, &to, withdraw_amount);
                add_token_collateral_amount(&e, &token, withdraw_amount);
                transfer_a(&e, &to, withdraw_amount);
            } else {
                put_withdrawn_collateral(&e, &to, withdraw_amount);
                add_token_collateral_amount(&e, &token, withdraw_amount);
                transfer_b(&e, &to, withdraw_amount);
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

    fn liq_adm(e: Env, to: Address, from: Address, spot_price: i128) -> Result<i128, Error> {
        from.require_auth();

        if !is_authorized(&e, &from) {
            return Err(Error::Unauthorized);
        }

        Ok(liquidate_user(&e, &to, &from, spot_price))
    }

    fn repay(e: Env, to: Address, token: Address, amount: i128) -> Result<(i128, i128), Error> {
        to.require_auth();

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
        }

        if is_liquidated(&e, &to) {
            return Err(Error::LiquidatedUser);
        }

        let deposited_token = get_deposited_token(&e, &to).unwrap();
        if deposited_token == get_token_a_address(&e) {
            if token != get_token_b_address(&e) {
                return Err(Error::WrongRepayToken);
            }
        } else {
            if token != get_token_a_address(&e) {
                return Err(Error::WrongRepayToken);
            }
        }

        let prev_total_amount_to_repay = get_user_amount_to_repay(&e, &to);
        let prev_total_returned_amount = get_returned_amount(&e, &to);
        let repay_amount = min(
            amount,
            prev_total_amount_to_repay - prev_total_returned_amount,
        );

        if repay_amount <= 0 {
            return Err(Error::AlreadyRepaid);
        }

        token::Client::new(&e, &token).transfer(&to, &e.current_contract_address(), &repay_amount);
        put_returned_amount(&e, &to, repay_amount);
        add_token_returned_amount(&e, &token, repay_amount);

        let total_returned_amount = get_returned_amount(&e, &to);
        let total_amount_to_repay = get_user_amount_to_repay(&e, &to);
        Ok((total_returned_amount, total_amount_to_repay))
    }

    fn withdraw(e: Env, to: Address) -> Result<i128, Error> {
        let forward_rate = get_forward_rate(&e);
        let spot_rate = get_spot_rate(&e);
        let returned_amount = get_returned_amount(&e, &to);
        let swapped_amount = get_swapped_amount(&e, &to);
        let deposited_token = get_deposited_token(&e, &to).unwrap();
        let withdraw_amount: i128;

        if !max_time_reached(&e) {
            return Err(Error::TimeNotReached);
        }

        if deposited_token == get_token_a_address(&e) {
            let used_deposited_amount = convert_amount_token_b_to_a(swapped_amount, forward_rate);
            let converted_returned_amount =
                convert_amount_token_b_to_a(returned_amount, forward_rate);
            withdraw_amount = min(used_deposited_amount, converted_returned_amount);

            add_token_withdrawn_amount(&e, &deposited_token, withdraw_amount);
            put_withdrawn_amount(&e, &to, withdraw_amount);
            transfer_a(&e, &to, withdraw_amount);
        } else {
            // User returned token_a
            let used_deposited_amount = convert_amount_token_a_to_b(swapped_amount, spot_rate);
            let converted_returned_amount =
                convert_amount_token_a_to_b(returned_amount, forward_rate);
            withdraw_amount = min(used_deposited_amount, converted_returned_amount);

            add_token_withdrawn_amount(&e, &deposited_token, withdraw_amount);
            put_withdrawn_amount(&e, &to, withdraw_amount);
            transfer_b(&e, &to, withdraw_amount);
        }

        Ok(withdraw_amount)
    }

    fn spot_rate(e: Env) -> i128 {
        get_spot_rate(&e)
    }

    // See admin
    fn admin(e: Env) -> Address {
        get_admin(&e).unwrap()
    }

    fn near_leg(e: Env) -> Result<PriceData, Error> {
        if !near_leg_time_reached(&e) {
            return Err(Error::ExecutionTimeNotReached);
        }

        let spot_rate: i128 = get_spot_rate(&e);
        if spot_rate != 0 {
            return Err(Error::SpotRateAlreadyDefined);
        }

        Ok(exec_near_leg(&e))
    }

    fn tokens(e: Env) -> (Token, Token) {
        let token_a = get_token_a(&e);
        let token_b = get_token_b(&e);
        (token_a, token_b)
    }

    fn set_spot(e: Env, to: Address, amount: i128) -> Result<(), Error> {
        to.require_auth();

        if !is_authorized(&e, &to) {
            return Err(Error::Unauthorized);
        }

        put_spot_rate(&e, amount);
        Ok(())
    }

    fn state(e: Env) -> State {
        get_state(&e)
    }

    fn deposits(e: Env) -> (Vec<Position>, Vec<Position>) {
        (get_used_positions_a(&e), get_used_positions_b(&e))
    }
}
