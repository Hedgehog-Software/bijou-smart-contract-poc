#![cfg(test)]
extern crate std;

use crate::constants::{ADMIN_ADDRESS, TIME_TO_EXEC, TIME_TO_MATURE, TIME_TO_REPAY};
use crate::storage_types::User;
use crate::SwapClient;

use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{symbol_short, token, vec, Address, Env, String};
use token::Client as TokenClient;

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let addr = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

struct SwapTest<'a> {
    e: Env,
    token_admin: Address,
    user_a: Address,
    user_b: Address,
    token_a: TokenClient<'a>,
    token_b: TokenClient<'a>,
    contract: SwapClient<'a>,
}

impl<'a> SwapTest<'a> {
    fn setup() -> Self {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().with_mut(|li| {
            li.timestamp = 12345;
        });

        let user_a = Address::generate(&e);
        let user_b = Address::generate(&e);
        let token_admin = Address::generate(&e);

        let (token_a, token_admin_client_a) = create_token_contract(&e, &token_admin);
        token_admin_client_a.mint(&user_a, &1_000);
        let (token_b, token_admin_client_b) = create_token_contract(&e, &token_admin);
        token_admin_client_b.mint(&user_b, &1_000);

        let contract = SwapClient::new(&e, &e.register_contract(None, crate::Swap {}));
        SwapTest {
            e,
            token_admin,
            user_a,
            user_b,
            token_a,
            token_b,
            contract,
        }
    }

    fn add_time(e: &Env, time: u64) {
        let blocks = time / 5;
        let ledger = e.ledger();
        e.ledger().set(LedgerInfo {
            timestamp: ledger.timestamp().saturating_add(time),
            protocol_version: ledger.protocol_version(),
            sequence_number: ledger.sequence().saturating_add(blocks as u32),
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 999999,
            min_temp_entry_ttl: 999999,
            max_entry_ttl: u32::MAX,
        });
    }

    fn mint_token() {}
}

#[test]
fn test_init() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a: _,
        user_b: _,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
}

#[test]
fn test_deposit() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    let (amount_a,deposit_a) = contract.deposit(&user_a, &token_a.address, &100, &10);
    let (amount_b, deposit_b) = contract.deposit(&user_b, &token_b.address, &200, &20);

    assert_eq!(amount_a, 100);
    assert_eq!(amount_b, 200);
    assert_eq!(deposit_a, 10);
    assert_eq!(deposit_b, 20);

    assert_eq!(token_a.balance(&user_a), 890);
    assert_eq!(token_b.balance(&user_b), 780);
}

// #[test]
// fn test_swap() {
//     let forward_rate: i128 = 100_000;
//     let SwapTest {
//         e: _,
//         token_admin: _,
//         user_a,
//         user_b,
//         token_a,
//         token_b,
//         contract,
//     } = SwapTest::setup();
//     contract.initialize(&token_a.address, &token_b.address, &forward_rate);
//     contract.deposit(&user_a, &token_a.address, &100, &10);
//     contract.deposit(&user_b, &token_b.address, &200, &20);

//     assert_eq!(token_b.balance(&user_a), 0);
//     contract.swap(&user_a);
//     assert_eq!(token_b.balance(&user_a), 100);
// }

// #[test]
// fn test_time() {
//     let forward_rate: i128 = 100_000;
//     let SwapTest {
//         e,
//         token_admin: _,
//         user_a:_,
//         user_b:_,
//         token_a,
//         token_b,
//         contract,
//     } = SwapTest::setup();
//     let ledger = e.ledger();
//     let time = 10000;
//     let blocks = time / 5;
//     e.ledger().set(LedgerInfo {
//         timestamp: ledger.timestamp().saturating_add(time),
//         protocol_version: ledger.protocol_version(),
//         sequence_number:
//         ledger.sequence().saturating_add(blocks as u32),
//         network_id: Default::default(),
//         base_reserve: 10,
//         min_persistent_entry_ttl: 999999,
//         min_temp_entry_ttl: 999999,
//         max_entry_ttl: u32::MAX
//     });

//     contract.initialize(&token_a.address, &token_b.address, &forward_rate);
//     let t_0 = contract.time();
//     assert_eq!(t_0, 100);
// }

#[test]
fn test_near_leg() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a: _,
        user_b: _,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    let price_data = contract.near_leg();
    assert_eq!(price_data.price, 100_000);
}

#[test]
fn test_swap() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);

    contract.near_leg();

    assert_eq!(token_b.balance(&user_a), 0);
    let swapped_amount = contract.swap(&user_a);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_a), 100);
}

#[test]
fn test_repay_a() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_a);

    assert_eq!(token_b.balance(&user_a), 100);
    let (repaid,total_amount_to_repay) = contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(repaid,100);
    assert_eq!(total_amount_to_repay,100);
    assert_eq!(token_b.balance(&user_a), 0);
}

#[test]
fn test_repay_b() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_b);

    assert_eq!(token_a.balance(&user_b), 100);
    let (repaid,total_amount_to_repay) = contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(repaid,100);
    assert_eq!(total_amount_to_repay,100);
    assert_eq!(token_a.balance(&user_b), 0);
}

#[test]
fn test_balance() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_a);
    contract.repay(&user_a, &token_b.address, &100);
    let balance = contract.balance(&user_a);
    assert_eq!(
        balance,
        User {
            deposited_token: token_a.address,
            deposited_amount: 100,
            swapped_amount: 100,
            returned_amount: 100,
            refunded_amount: 0,
            withdrawn_amount: 0,
            collateral: 10,
            is_liquidated: false,
        }
    );
}

#[test]
fn test_withdraw_a() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_a);
    contract.swap(&user_b);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_a.balance(&user_a), 890);
    assert_eq!(token_b.balance(&user_a), 0);
    let withdrawn_amount = contract.withdraw(&user_a);
    assert_eq!(withdrawn_amount, 100);
    assert_eq!(token_a.balance(&user_a), 990);
}

#[test]
fn test_withdraw_b() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_a);
    contract.swap(&user_b);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 780);
    let withdrawn_amount = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount, 100);
    assert_eq!(token_b.balance(&user_b), 880);
}

#[test]
fn test_reclaim() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.near_leg();
    let swap_amount = contract.swap(&user_b);
    assert_eq!(swap_amount, 100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 780);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 880);

    SwapTest::add_time(&e, TIME_TO_MATURE + TIME_TO_REPAY);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 980);
}

#[test]
fn test_reclaim_collateral() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e: _,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.near_leg();
    contract.swap(&user_a);
    contract.repay(&user_a, &token_b.address, &100);
    contract.withdraw(&user_a);
    assert_eq!(token_a.balance(&user_a), 990);

    let reclaimed_collateral = contract.reclaim_col(&user_a);
    assert_eq!(reclaimed_collateral, 10);
    assert_eq!(token_a.balance(&user_a), 1000);
}

#[test]
fn test_liquidate_swap() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.near_leg();
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liquidate(&user_a, &user_b);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_a.balance(&user_b), 901);
}

#[test]
fn test_liquidate_repay() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        e,
        token_admin: _,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
    } = SwapTest::setup();
    contract.initialize(&token_a.address, &token_b.address, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.near_leg();
    contract.swap(&user_a);
    contract.swap(&user_b);
    contract.repay(&user_a, &token_b.address, &800);
    SwapTest::add_time(&e, TIME_TO_MATURE + TIME_TO_REPAY);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liquidate(&user_a, &user_b);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_a.balance(&user_b), 901);
}

// #[test]
// fn test_different_spot_rate() {
//     let forward_rate: i128 = 100_000;
//     let SwapTest {
//         e,
//         token_admin: _,
//         user_a,
//         user_b,
//         token_a,
//         token_b,
//         contract,
//     } = SwapTest::setup();
//     let spot_rate: i128 = 150_000; // 1 Token A = 1.5 token B
//     let admin_address = Address::from_string(&String::from_str(&e, ADMIN_ADDRESS));

//     contract.initialize(&token_a.address, &token_b.address, &forward_rate);
//     contract.deposit(&user_a, &token_a.address, &100, &15);
//     contract.deposit(&user_b, &token_b.address, &200, &30);
//     contract.set_spot(&user_a, &spot_rate);

//     contract.swap(&user_a);
//     contract.swap(&user_b);

//     assert_eq!(token_b.balance(&user_a), 150);
//     assert_eq!(token_a.balance(&user_b), 100);

//     contract.repay(&user_a, &token_b.address, &150);
//     contract.repay(&user_b, &token_a.address, &100);

//     let withdrawn_amount_a = contract.withdraw(&user_a);
//     let withdrawn_amount_b = contract.withdraw(&user_b);
//     assert_eq!(withdrawn_amount_a, 100);
//     assert_eq!(withdrawn_amount_b, 150);
//     assert_eq!(token_a.balance(&user_a), 985);
//     assert_eq!(token_b.balance(&user_b), 970);
// }
