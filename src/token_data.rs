use soroban_sdk::{Address, Env, Symbol};
use types::{storage::DataKey, token::Token};

use crate::types;

pub(crate) fn init_token_a(e: &Env, token: &Address, name: Symbol) {
    e.storage().instance().set(
        &DataKey::TokenA,
        &Token {
            name,
            address: token.clone(),
            deposited_amount: 0,
            swapped_amount: 0,
            returned_amount: 0,
            withdrawn_amount: 0,
            collateral_amount: 0,
            withdrawn_collateral: 0,
        },
    );
}

pub(crate) fn init_token_b(e: &Env, token: &Address, name: Symbol) {
    e.storage().instance().set(
        &DataKey::TokenB,
        &Token {
            name,
            address: token.clone(),
            deposited_amount: 0,
            swapped_amount: 0,
            returned_amount: 0,
            withdrawn_amount: 0,
            collateral_amount: 0,
            withdrawn_collateral: 0,
        },
    );
}

pub(crate) fn get_token_a(e: &Env) -> Token {
    e.storage().instance().get(&DataKey::TokenA).unwrap()
}

pub(crate) fn get_token_b(e: &Env) -> Token {
    e.storage().instance().get(&DataKey::TokenB).unwrap()
}

fn get_token(e: &Env, token: &Address) -> Token {
    let token_a_address = get_token_a_address(&e);
    let key = match token_a_address == token.clone() {
        true => DataKey::TokenA,
        false => DataKey::TokenB,
    };
    e.storage().instance().get(&key).unwrap()
}

fn edit_token(e: &Env, token: &Address, data: Token) {
    let token_a_address = get_token_a_address(&e);
    let key = match token_a_address == token.clone() {
        true => DataKey::TokenA,
        false => DataKey::TokenB,
    };
    e.storage().instance().set(&key, &data);
}

pub(crate) fn get_token_a_address(e: &Env) -> Address {
    let token_data: Token = get_token_a(&e);
    token_data.address
}

pub(crate) fn get_token_b_address(e: &Env) -> Address {
    let token_data: Token = get_token_b(&e);
    token_data.address
}

pub(crate) fn add_token_deposited_amount(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.deposited_amount += amount;
    edit_token(e, &token, token_data);
}

pub(crate) fn add_token_swapped_amount(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.swapped_amount += amount;
    edit_token(e, &token, token_data);
}

pub(crate) fn add_token_returned_amount(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.returned_amount += amount;
    edit_token(e, &token, token_data);
}

pub(crate) fn add_token_withdrawn_amount(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.withdrawn_amount += amount;
    edit_token(e, &token, token_data);
}

pub(crate) fn add_token_collateral_amount(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.collateral_amount += amount;
    edit_token(e, &token, token_data);
}

pub(crate) fn add_token_withdrawn_collateral(e: &Env, token: &Address, amount: i128) {
    let mut token_data = get_token(&e, &token);
    token_data.withdrawn_collateral += amount;
    edit_token(e, &token, token_data);
}
