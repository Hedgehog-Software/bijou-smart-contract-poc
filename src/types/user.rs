use soroban_sdk::{contracttype, Address};

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct User {
    pub deposited_token: Address,
    pub deposited_amount: i128,
    pub swapped_amount: i128,
    pub returned_amount: i128,
    pub withdrawn_amount: i128,
    pub refunded_amount: i128,
    pub collateral: i128,
    pub is_liquidated: bool,
}
