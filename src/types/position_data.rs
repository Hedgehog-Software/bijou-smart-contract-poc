use soroban_sdk::contracttype;

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct PositionData {
    pub limit: u64,
    pub used: u64,
    pub deposit_amount: i128,
}
