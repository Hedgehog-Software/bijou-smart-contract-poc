use soroban_sdk::{contracttype, Address};

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct UserLiqData {
    pub address: Address,
    pub collateral: i128,
    pub min_collateral: i128,
    pub is_liquidated: bool,
}
