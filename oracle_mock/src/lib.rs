#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

const RATE: Symbol = symbol_short!("RATE");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

pub trait OracleMockTrait {
    fn set_spot_rate(e: Env, spot_rate: i128) -> PriceData;

    fn x_last_price(e: Env, base_asset: Asset, quote_asset: Asset) -> Option<PriceData>;
}

#[contract]
struct OracleMock;

#[contractimpl]
impl OracleMockTrait for OracleMock {
    fn set_spot_rate(e: Env, spot_rate: i128) -> PriceData {
        e.storage().instance().set(&RATE, &spot_rate);
        PriceData {
            price: spot_rate,
            timestamp: 0,
        }
    }

    fn x_last_price(e: Env, base_asset: Asset, quote_asset: Asset) -> Option<PriceData> {
        let price = e.storage().instance().get(&RATE).unwrap_or(0);
        Some(PriceData {
            price,
            timestamp: 0,
        })
    }
}
