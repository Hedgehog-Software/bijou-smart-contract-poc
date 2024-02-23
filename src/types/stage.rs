use soroban_sdk::contracttype;

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Stage {
    Deposit = 1,
    Swap = 2,
    Repay = 3,
    Withdraw = 4,
}
