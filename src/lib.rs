#![no_std]

mod constants;
mod storage_types;
mod test;
mod token_data;

use core::cmp::min;

use constants::{COLLATERAL_BUFFER, ORACLE_ADDRESS, SCALE, TIME_TO_EXEC, TIME_TO_REPAY};
use soroban_sdk::{contract, contractimpl, token, vec, Address, Env, String, Symbol};
use storage_types::{Asset, DataKey, Error, PriceData, Token, User};
use token_data::{
    add_token_collateral_amount, add_token_deposited_amount, add_token_returned_amount,
    add_token_swapped_amount, add_token_withdrawn_amount, get_token_a, get_token_a_address,
    get_token_b, get_token_b_address, init_token_a, init_token_b,
};

fn get_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&DataKey::Admin)
}

fn get_and_add(e: &Env, key: DataKey, amount: i128) {
    let mut count: i128 = e.storage().persistent().get(&key).unwrap_or_default();
    count += amount;
    e.storage().persistent().set(&key, &count);
}

fn get_deposited_token(e: &Env, to: &Address) -> Option<Address> {
    let key = DataKey::DepositedToken(to.clone());
    e.storage().persistent().get(&key)
}

fn get_deposited_amount(e: &Env, to: &Address) -> i128 {
    let key = DataKey::DepositedAmount(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

fn get_collateral(e: &Env, to: &Address) -> i128 {
    let key = DataKey::Collateral(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

fn get_withdrawn_collateral(e: &Env, to: &Address) -> i128 {
    let key = DataKey::WithdrawnCollateralAmount(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

fn get_returned_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::ReturnedAmount(to.clone()))
        .unwrap_or_default()
}

fn get_swapped_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::SwappedAmount(to.clone()))
        .unwrap_or_default()
}

fn get_withdrawn_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::WithdrawnAmount(to.clone()))
        .unwrap_or_default()
}

fn get_is_liquidated(e: &Env, to: &Address) -> bool {
    e.storage()
        .persistent()
        .get(&DataKey::IsLiquidated(to.clone()))
        .unwrap_or(false)
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

fn put_deposited_token(e: &Env, to: &Address, token: &Address) {
    e.storage()
        .persistent()
        .set(&DataKey::DepositedToken(to.clone()), &token);
}

fn put_forward_rate(e: &Env, rate: i128) {
    e.storage().persistent().set(&DataKey::ForwardRate, &rate);
}

fn put_spot_rate(e: &Env, amount: i128) {
    e.storage().persistent().set(&DataKey::SpotRate, &amount);
}

fn put_deposited_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::DepositedAmount(to.clone());
    get_and_add(e, key, amount);
}

fn put_swapped_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::SwappedAmount(to.clone());
    get_and_add(e, key, amount);
}

fn put_collateral(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::Collateral(to.clone());
    get_and_add(e, key, amount);
}

fn put_withdrawn_collateral(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::WithdrawnCollateralAmount(to.clone());
    get_and_add(e, key, amount);
}

fn put_withdrawn_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::WithdrawnAmount(to.clone());
    get_and_add(e, key, amount);
}

fn put_returned_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::ReturnedAmount(to.clone());
    get_and_add(e, key, amount);
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

fn has_not_repaid(e: &Env, to: &Address) -> bool {
    let swapped_amount = get_swapped_amount(&e, &to);
    let returned_amount = get_returned_amount(&e, &to);
    swapped_amount >= returned_amount
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
            let min_collateral =
                convert_amount_token_b_to_a(COLLATERAL_BUFFER * swapped_amount, forward_rate);
            let curr_collateral = convert_amount_token_b_to_a(collateral, spot_price);
            reward_amount = collateral / 100;

            if (min_collateral > curr_collateral) || expired_and_not_repaid {
                put_is_liquidated(&e, &to, true);
                transfer_a(&e, &from, reward_amount);
            }
        } else {
            let min_collateral =
                convert_amount_token_a_to_b(COLLATERAL_BUFFER * swapped_amount, forward_rate);
            let curr_collateral = convert_amount_token_a_to_b(collateral, spot_price);
            reward_amount = collateral / 100;

            if (min_collateral > curr_collateral) || expired_and_not_repaid {
                put_is_liquidated(&e, &to, true);
                transfer_b(&e, &from, reward_amount);
            }
        }
    }
    reward_amount
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

fn get_user_balance(e: &Env, to: &Address) -> User {
    let deposited_token = get_deposited_token(&e, &to).unwrap();
    User {
        deposited_token,
        deposited_amount: get_deposited_amount(&e, &to),
        swapped_amount: get_swapped_amount(&e, &to),
        returned_amount: get_returned_amount(&e, &to),
        withdrawn_amount: get_withdrawn_amount(&e, &to),
        refunded_amount: 0,
        collateral: get_collateral(&e, &to),
        is_liquidated: get_is_liquidated(&e, &to),
    }
}

fn get_user_deposit(e: &Env, to: &Address) -> (i128, i128) {
    let deposited_amount = get_deposited_amount(&e, &to);
    let collateral = get_collateral(&e, &to);
    (deposited_amount, collateral)
}

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
    // None
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
}

// Users can monitor the contract
// ForwardRate and SpotRate includes the scale
// Initiation  State
// - Users can deposit amount and collateral
// -
// After x time contract gets in the execition state (Fixed now, custom in the future)
// > Get spot rate
// > Change state
// Execution state
// - User can deposit collateral only to mantain its position
// - User can be liqudated
// - User can swap currency (Using spot rate)
// -
// After Exp time contract reaches maturity
// - User has 48 hours to deposit
// - User must deposit the swapped amount (Using forward rate)
// - User can deposit colateral
// - User can be liqudated
// - User can withdraw its initial deposit after having deposited the swapped
// - User can refund (Excess in the deposited currency) (And clolateral)
//
// After 48 hours
// - User can withdraw the original amount anytime
//

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

    fn deposit(
        e: Env,
        to: Address,
        token: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(i128, i128), Error> {
        // Depositor needs to authorize the deposit
        to.require_auth();
        let near_leg_executed = has_near_leg_executed(&e);

        if near_leg_executed && amount != 0 {
            return Err(Error::CollateralOnlyCanBeDeposited);
        }

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
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
            token::Client::new(&e, &token).transfer(&to, &e.current_contract_address(), &amount);
            put_deposited_amount(&e, &to, amount);
            add_token_deposited_amount(&e, &token, amount);
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
                let deposited_amount = get_deposited_amount(&e, &to);
                let exp_swap_amount = convert_amount_token_a_to_b(deposited_amount, spot_rate);

                let token_b_data = get_token_b(&e);
                let token_b_available_amount =
                    token_b_data.deposited_amount - token_b_data.swapped_amount;

                swap_amount = min(exp_swap_amount, token_b_available_amount);
                put_swapped_amount(&e, &to, swap_amount);
                add_token_swapped_amount(&e, &token_b_data.address, swap_amount);
                transfer_b(&e, &to, swap_amount);
            } else {
                let deposited_amount = get_deposited_amount(&e, &to);
                let exp_swap_amount = convert_amount_token_b_to_a(deposited_amount, spot_rate);

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

        if get_is_liquidated(&e, &to) {
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

        // TODO: Avoid user placing more money than expected

        if !is_valid_token(&e, token.clone()) {
            return Err(Error::InvalidToken);
        }

        if get_is_liquidated(&e, &to) {
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
}
