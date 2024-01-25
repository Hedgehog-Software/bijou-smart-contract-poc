use soroban_sdk::{Address, Env, Vec};
use storage_types::DataKey;
use types::position::Position;

use crate::storage_types;
use crate::token_data::get_token_a_address;
use crate::types;

fn get_used_positions(e: &Env, token: &Address) -> Vec<Position> {
    let token_a_address = get_token_a_address(&e);
    let key = match token_a_address == token.clone() {
        true => DataKey::UsedPositionsA,
        false => DataKey::UsedPositionsB,
    };
    e.storage().persistent().get(&key).unwrap_or(Vec::new(&e))
}

fn put_used_positions(e: &Env, token: &Address, used_positions: &Vec<Position>) {
    let token_a_address = get_token_a_address(&e);
    let key = match token_a_address == token.clone() {
        true => DataKey::UsedPositionsA,
        false => DataKey::UsedPositionsB,
    };
    e.storage().persistent().set(&key, used_positions);
}

pub fn create_position(e: &Env, to: &Address, token: &Address) -> u32 {
    let position = Position {
        address: to.clone(),
        is_valid: false,
    };
    let mut used_position = get_used_positions(&e, &token);
    used_position.push_back(position);
    put_used_positions(&e, &token, &used_position);
    used_position.len() - 1
}

pub fn set_position_valid(e: &Env, position_index: u32, token: &Address) {
    let mut used_position = get_used_positions(&e, &token);
    let mut position = used_position.get(0).unwrap();
    position.is_valid = true;
    used_position.set(position_index, position);
    put_used_positions(&e, &token, &used_position);
}
