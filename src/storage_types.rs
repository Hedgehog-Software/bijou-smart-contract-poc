use soroban_sdk::{contracterror, contracttype, Address, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    DifferentDepositedToken = 1,
    WrongRepayToken = 2,
    ExecutionTimeNotReached = 3,
    SpotRateAlreadyDefined = 4,
    LiquidatedUser = 5,
    TimeNotReached = 6,
    CollateralOnlyCanBeDeposited = 7,
    NearLegNotExecuted = 8,
    InvalidToken = 9,
    ContractStillOpen = 10,
    AlreadyRepaid = 11,
    Unauthorized = 12,
    ContractAlreadyInitialized = 13,
}

// pub enum State {
//     Initiation = 1,
//     Execution = 2,
//     Completion = 3,
// }

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum State {
    Deposit = 1,
    Swap = 2,
    Repay = 3,
    Withdraw = 4,
}

//quoted asset definition
#[contracttype]
pub enum Asset {
    Stellar(Address), //for Stellar Classic and Soroban assets
    Other(Symbol),    //for any external tokens/assets/symbols
}

//price record definition
#[contracttype]
pub struct PriceData {
    pub price: i128,    //asset price at given point in time
    pub timestamp: u64, //recording timestamp
}

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

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct Token {
    pub name: Symbol,
    pub address: Address,
    pub deposited_amount: i128,
    pub swapped_amount: i128,
    pub returned_amount: i128,
    pub withdrawn_amount: i128,
    pub collateral_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    TokenA,
    TokenB,
    Exp,
    SpotRate,
    ForwardRate,
    InitTime,
    TimeToMature,
    DepositedToken(Address),
    DepositedAmount(Address),
    Collateral(Address),
    SwappedAmount(Address),
    ReturnedAmount(Address),
    WithdrawnAmount(Address),
    WithdrawnCollateralAmount(Address),
    RefundedAmount(Address),
    IsLiquidated(Address),
}
