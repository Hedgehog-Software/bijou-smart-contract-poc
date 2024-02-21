use soroban_sdk::{Address, Env, Vec};
use types::position::Position;

use crate::token_data::get_token_a_address;
use crate::types::{self, storage::DataKey};

pub(crate) fn get_used_positions_a(e: &Env) -> Vec<Position> {
    e.storage()
        .persistent()
        .get(&DataKey::UsedPositionsA)
        .unwrap_or(Vec::new(&e))
}

pub(crate) fn get_used_positions_b(e: &Env) -> Vec<Position> {
    e.storage()
        .persistent()
        .get(&DataKey::UsedPositionsB)
        .unwrap_or(Vec::new(&e))
}

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

pub(crate) fn create_position(e: &Env, to: &Address, token: &Address) -> u32 {
    let position = Position {
        address: to.clone(),
        is_valid: false,
    };
    let mut used_position = get_used_positions(&e, &token);
    used_position.push_back(position);
    put_used_positions(&e, &token, &used_position);
    used_position.len() - 1
}

pub(crate) fn set_position_valid(e: &Env, position_index: u32, token: &Address) {
    let mut used_position = get_used_positions(&e, &token);
    let mut position = used_position.get(position_index).unwrap();
    position.is_valid = true;
    used_position.set(position_index, position);
    put_used_positions(&e, &token, &used_position);
}
