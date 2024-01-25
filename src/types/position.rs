use soroban_sdk::{contracttype, Address};

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct Position {
    pub address: Address,
    pub is_valid: bool,
}
