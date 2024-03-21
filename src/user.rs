use soroban_sdk::{Address, Env};
use types::{storage::DataKey, user::User};

use crate::types;

fn get_and_add(e: &Env, key: DataKey, amount: i128) {
    let count: i128 = e.storage().persistent().get(&key).unwrap_or_default();
    e.storage().persistent().set(&key, &(count + amount));
}

pub(crate) fn get_deposited_token(e: &Env, to: &Address) -> Option<Address> {
    let key = DataKey::DepositedToken(to.clone());
    e.storage().persistent().get(&key)
}

pub(crate) fn get_deposited_amount(e: &Env, to: &Address) -> i128 {
    let key = DataKey::DepositedAmount(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

pub(crate) fn get_collateral(e: &Env, to: &Address) -> i128 {
    let key = DataKey::Collateral(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

pub(crate) fn get_withdrawn_collateral(e: &Env, to: &Address) -> i128 {
    let key = DataKey::WithdrawnCollateralAmount(to.clone());
    e.storage().persistent().get(&key).unwrap_or_default()
}

pub(crate) fn get_returned_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::ReturnedAmount(to.clone()))
        .unwrap_or_default()
}

pub(crate) fn get_swapped_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::SwappedAmount(to.clone()))
        .unwrap_or_default()
}

pub(crate) fn get_withdrawn_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::WithdrawnAmount(to.clone()))
        .unwrap_or_default()
}

pub(crate) fn get_reclaimed_amount(e: &Env, to: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::ReclaimedAmount(to.clone()))
        .unwrap_or_default()
}

pub(crate) fn is_liquidated(e: &Env, to: &Address) -> bool {
    e.storage()
        .persistent()
        .get(&DataKey::IsLiquidated(to.clone()))
        .unwrap_or(false)
}

pub(crate) fn get_user_deposit(e: &Env, to: &Address) -> (i128, i128) {
    let deposited_amount = get_deposited_amount(&e, &to);
    let collateral = get_collateral(&e, &to);
    (deposited_amount, collateral)
}

pub(crate) fn get_user_balance(e: &Env, to: &Address) -> User {
    let deposited_token = get_deposited_token(&e, &to).unwrap();
    User {
        deposited_token,
        deposited_amount: get_deposited_amount(&e, &to),
        swapped_amount: get_swapped_amount(&e, &to),
        returned_amount: get_returned_amount(&e, &to),
        withdrawn_amount: get_withdrawn_amount(&e, &to),
        reclaimed_amount: get_reclaimed_amount(&e, &to),
        collateral: get_collateral(&e, &to),
        withdrawn_collateral: get_withdrawn_collateral(&e, &to),
        is_liquidated: is_liquidated(&e, &to),
    }
}

pub(crate) fn put_deposited_token(e: &Env, to: &Address, token: &Address) {
    e.storage()
        .persistent()
        .set(&DataKey::DepositedToken(to.clone()), &token);
}

pub(crate) fn put_deposited_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::DepositedAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_swapped_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::SwappedAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_collateral(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::Collateral(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_withdrawn_collateral(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::WithdrawnCollateralAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_withdrawn_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::WithdrawnAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_returned_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::ReturnedAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_reclaimed_amount(e: &Env, to: &Address, amount: i128) {
    let key = DataKey::ReclaimedAmount(to.clone());
    get_and_add(e, key, amount);
}

pub(crate) fn put_is_liquidated(e: &Env, to: &Address, val: bool) {
    let key = DataKey::IsLiquidated(to.clone());
    e.storage().persistent().set(&key, &val);
}

pub(crate) fn has_not_repaid(e: &Env, to: &Address) -> bool {
    let swapped_amount = get_swapped_amount(&e, &to);
    let returned_amount = get_returned_amount(&e, &to);
    swapped_amount >= returned_amount
}
