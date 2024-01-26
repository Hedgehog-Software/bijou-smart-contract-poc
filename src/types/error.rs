use soroban_sdk::contracterror;

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
    PositionsAlreadyInitialized = 14,
    DepositAmountDoesntMatchPosition = 15,
    AllPositionsAreUsed = 16,
    NotEnoughtPositionsUsed = 17
}