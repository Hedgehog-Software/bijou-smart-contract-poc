use soroban_sdk::{contracttype, Address, Symbol};

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct Token {
    pub name: Symbol,
    pub address: Address,
    pub deposited_amount: i128,
    pub swapped_amount: i128,
    pub returned_amount: i128,
    pub withdrawn_amount: i128,
    pub reclaimed_amount: i128,
    pub collateral_amount: i128,
    pub withdrawn_collateral: i128,
}
