use soroban_sdk::{Address, Env};
use types::storage::DataKey;

use crate::types;

pub(crate) fn get_admin(e: &Env) -> Option<Address> {
    e.storage().instance().get(&DataKey::Admin)
}

pub(crate) fn get_spot_rate(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&DataKey::SpotRate)
        .unwrap_or_default()
}

pub(crate) fn get_forward_rate(e: &Env) -> i128 {
    e.storage().instance().get(&DataKey::ForwardRate).unwrap()
}

pub(crate) fn get_init_time(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::InitTime).unwrap()
}

pub(crate) fn get_time_to_mature(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::TimeToMature).unwrap()
}

pub(crate) fn put_admin(e: &Env, address: Address) {
    e.storage().instance().set(&DataKey::Admin, &address);
}

pub(crate) fn put_forward_rate(e: &Env, rate: i128) {
    e.storage().instance().set(&DataKey::ForwardRate, &rate);
}

pub(crate) fn put_spot_rate(e: &Env, amount: i128) {
    e.storage().instance().set(&DataKey::SpotRate, &amount);
}

pub(crate) fn put_init_time(e: &Env) {
    let time = e.ledger().timestamp();
    e.storage().instance().set(&DataKey::InitTime, &time);
}

pub(crate) fn put_time_to_mature(e: &Env, duration: u64) {
    e.storage()
        .instance()
        .set(&DataKey::TimeToMature, &duration);
}
