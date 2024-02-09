use soroban_sdk::{Address, Env};

use crate::token_data::get_token_a_address;
use crate::types;
use types::{position_data::PositionData, storage::DataKey};

pub fn init_position_a(e: &Env, limit: u64, amount: i128) {
    e.storage().instance().set(
        &DataKey::PositionA,
        &PositionData {
            limit,
            used: 0,
            deposit_amount: amount,
        },
    );
}

pub fn init_position_b(e: &Env, limit: u64, amount: i128) {
    e.storage().instance().set(
        &DataKey::PositionB,
        &PositionData {
            limit,
            used: 0,
            deposit_amount: amount,
        },
    );
}

pub fn set_position_a(e: &Env, position: &PositionData) {
    e.storage().instance().set(&DataKey::PositionA, position);
}

pub fn set_position_b(e: &Env, position: &PositionData) {
    e.storage().instance().set(&DataKey::PositionB, position);
}

pub fn get_position_a(e: &Env) -> PositionData {
    e.storage().instance().get(&DataKey::PositionA).unwrap()
}

pub fn get_position_b(e: &Env) -> PositionData {
    e.storage().instance().get(&DataKey::PositionB).unwrap()
}

pub fn get_position_data(e: &Env, token: &Address) -> PositionData {
    match token.clone() == get_token_a_address(&e) {
        true => get_position_a(&e),
        false => get_position_b(&e),
    }
}

pub fn are_positions_open(position: &PositionData) -> bool {
    position.used < position.limit
}

pub fn ocupy_one_position(e: &Env, token: &Address, position: &PositionData) {
    let new_position_data = PositionData {
        limit: position.limit,
        used: position.used + 1,
        deposit_amount: position.deposit_amount,
    };
    match token.clone() == get_token_a_address(&e) {
        true => set_position_a(&e, &new_position_data),
        false => set_position_b(&e, &new_position_data),
    }
}
