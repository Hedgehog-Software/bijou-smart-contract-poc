use crate::constants::{ORACLE_ADDRESS, ORACLE_FUNCTION};
use crate::token_data::{get_token_a, get_token_b};
use soroban_sdk::{vec, Address, Env, String, Symbol};
use types::{asset::Asset, price_data::PriceData};

use crate::types;
pub fn get_oracle_spot_price(e: &Env) -> PriceData {
    let oracle_address: String = String::from_str(&e, ORACLE_ADDRESS);
    let target: Address = Address::from_string(&oracle_address);
    let func: Symbol = Symbol::new(&e, ORACLE_FUNCTION);
    let base_token = get_token_a(&e).name;
    let base_asset = Asset::Other(base_token);
    let quote_token = get_token_b(&e).name;
    let quote_asset = Asset::Other(quote_token);
    let args = vec![&e, base_asset, quote_asset].to_vals();
    e.invoke_contract::<PriceData>(&target, &func, args)
}
