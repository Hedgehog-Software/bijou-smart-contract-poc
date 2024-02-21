use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    TokenA,
    TokenB,
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
    ReclaimedAmount(Address),
    IsLiquidated(Address),
}
