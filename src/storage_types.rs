use soroban_sdk::{contracttype, Address};

// pub enum State {
//     Initiation = 1,
//     Execution = 2,
//     Completion = 3,
// }

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
    PositionA,
    PositionB,
    UsedPositionsA,
    UsedPositionsB,
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
