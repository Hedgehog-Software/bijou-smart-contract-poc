#![cfg(test)]
extern crate std;

use crate::constants::{SCALE, TIME_TO_EXEC, TIME_TO_MATURE, TIME_TO_REPAY};
use crate::storage_types::User;
use crate::SwapClient;

use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{symbol_short, token, Address, Env};
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
    token_admin_client_a: StellarAssetClient<'a>,
    token_admin_client_b: StellarAssetClient<'a>,
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
            token_admin_client_a,
            token_admin_client_b,
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

    // fn mint_token(token_admin_client: StellarAssetClient<'a>, to: &Address, amount: i128) {
    //     token_admin_client.mint(&to, &amount);
    // }
}

#[test]
fn test_init() {
    let forward_rate: i128 = 100_000_000_000_000;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
}

#[test]
fn test_deposit() {
    let forward_rate: i128 = 100_000;
    let SwapTest {
        user_a,
        user_b,
        token_admin,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    let (amount_a, collateral_a) = contract.deposit(&user_a, &token_a.address, &100, &10);
    let (amount_b, collateral_b) = contract.deposit(&user_b, &token_b.address, &200, &20);

    assert_eq!(amount_a, 100);
    assert_eq!(amount_b, 200);
    assert_eq!(collateral_a, 10);
    assert_eq!(collateral_b, 20);

    assert_eq!(token_a.balance(&user_a), 890);
    assert_eq!(token_b.balance(&user_b), 780);
}

#[test]
#[should_panic]
fn test_mature_date_error_withdraw() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        user_a,
        user_b,
        contract,
        ..
    } = SwapTest::setup();

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &10000,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.withdraw(&user_a);
}

#[test]
#[should_panic]
fn test_mature_date_error_reclaim_col() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        user_a,
        user_b,
        contract,
        ..
    } = SwapTest::setup();

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &10000,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.reclaim_col(&user_a);
}

#[test]
#[should_panic]
fn test_mature_date_error_reclaim() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        user_a,
        user_b,
        contract,
        ..
    } = SwapTest::setup();

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &10000,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.reclaim(&user_a);
}

// #[test]
// fn test_near_leg() {
//     let forward_rate: i128 = 100_000;
//     let SwapTest {
//         token_admin,
//         token_a,
//         token_b,
//         contract,
//         ..
//     } = SwapTest::setup();
//     contract.initialize(
//         &token_admin,
//         &token_a.address,
//         &token_b.address,
//         &symbol_short!("USDC"),
//         &symbol_short!("EURC"),
//         &forward_rate,
//         &0,
//     );
//     let price_data = contract.near_leg();
//     assert_eq!(price_data.price, 100_000);
// }

#[test]
fn test_set_spot_rate() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.set_spot(&token_admin, &forward_rate);
}

#[test]
#[should_panic]
fn test_set_spot_rate_unauthorized() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.set_spot(&user_a, &forward_rate);
}

#[test]
#[should_panic]
fn test_deposit_amount_after_near_leg() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.set_spot(&user_a, &forward_rate);
    contract.deposit(&user_a, &token_a.address, &100, &15);
}

#[test]
fn test_deposit_collateral_after_near_leg() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.set_spot(&token_admin, &forward_rate);
    let (amount_a, collateral_a) = contract.deposit(&user_a, &token_a.address, &0, &50);

    assert_eq!(amount_a, 100);
    assert_eq!(collateral_a, 60);
}

#[test]
fn test_swap() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);

    contract.set_spot(&token_admin, &forward_rate);

    assert_eq!(token_b.balance(&user_a), 0);
    let swapped_amount = contract.swap(&user_a);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_a), 100);
}

#[test]
fn test_repay_a() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);

    assert_eq!(token_b.balance(&user_a), 100);
    let (repaid, total_amount_to_repay) = contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(repaid, 100);
    assert_eq!(total_amount_to_repay, 100);
    assert_eq!(token_b.balance(&user_a), 0);
}

#[test]
fn test_repay_b() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);

    assert_eq!(token_a.balance(&user_b), 100);
    let (repaid, total_amount_to_repay) = contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(repaid, 100);
    assert_eq!(total_amount_to_repay, 100);
    assert_eq!(token_a.balance(&user_b), 0);
}

#[test]
#[should_panic]
fn test_repay_error() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);

    let (repaid, total_amount_to_repay) = contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(repaid, 100);
    assert_eq!(total_amount_to_repay, 100);
    contract.repay(&user_a, &token_b.address, &1);
}

#[test]
fn test_repay_instances() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);

    let (repaid, total_amount_to_repay) = contract.repay(&user_a, &token_b.address, &50);
    assert_eq!(repaid, 50);
    assert_eq!(total_amount_to_repay, 100);
    let (repaid, total_amount_to_repay) = contract.repay(&user_a, &token_b.address, &50);
    assert_eq!(repaid, 100);
    assert_eq!(total_amount_to_repay, 100);
}

#[test]
fn test_balance() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
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
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );

    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
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
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
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
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
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
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
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
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liq_adm(&user_a, &user_b, &forward_rate);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_a.balance(&user_b), 901);
}

#[test]
fn test_liquidate_repay() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    contract.repay(&user_a, &token_b.address, &800);
    SwapTest::add_time(&e, TIME_TO_MATURE + TIME_TO_REPAY);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liq_adm(&user_a, &user_b, &forward_rate);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_a.balance(&user_b), 901);
}

#[test]
fn test_forward_smaller_than_spot() {
    // user_a deposits 100, user_b deposits 200
    // user_a swaps and receives 150 of token_b, user_b receives 100 of token_a
    // because forward rate is 1 user_a
    //
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 191_000_000_000_000; // 1 Token A = 1.5 token B
    let forward_rate: i128 = 190_000_000_000_000;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(10_000 * decimals));
    token_admin_client_b.mint(&user_b, &(10_000 * decimals));

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("EURC"),
        &symbol_short!("USDC"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &(10_000 * decimals), &1_000);
    contract.deposit(&user_b, &token_b.address, &(10_000 * decimals), &1_000);
    contract.set_spot(&token_admin, &spot_rate);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_000_000_000_000);
    assert_eq!(swapped_amount_b, 523_560_209_424);

    assert_eq!(token_b.balance(&user_a), 1_000_000_000_000);
    assert_eq!(token_a.balance(&user_b), 523_560_209_424);

    token_admin_client_a.mint(&user_b, &2755580049);
    let repay_a = contract.repay(&user_a, &token_b.address, &1_000_000_000_000);
    let repay_b = contract.repay(&user_b, &token_a.address, &526_315_789_473);
    assert_eq!(repay_a, (1_000_000_000_000, 1_000_000_000_000));
    assert_eq!(repay_b, (526_315_789_473, 526_315_789_473));

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 526_315_789_473);
    assert_eq!(withdrawn_amount_b, 999_999_999_998);
    assert_eq!(token_a.balance(&user_a), 526_315_789_473);
    assert_eq!(token_b.balance(&user_b), 999_999_999_998);
}

#[test]
fn test_forward_bigger_than_spot() {
    // user_a deposits 100, user_b deposits 200
    // user_a swaps and receives 150 of token_b, user_b receives 100 of token_a
    // because forward rate is 1 user_a
    //
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 190_000_000_000_000; // 1 Token A = 1.5 token B
    let forward_rate: i128 = 191_000_000_000_000;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(10_000 * decimals));
    token_admin_client_b.mint(&user_b, &(10_000 * decimals));

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("GBP"),
        &symbol_short!("USD"),
        &forward_rate,
        &0,
    );
    contract.deposit(&user_a, &token_a.address, &(10_000 * decimals), &1_000);
    contract.deposit(&user_b, &token_b.address, &(10_000 * decimals), &1_000);
    contract.set_spot(&token_admin, &spot_rate);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_000_000_000_000);
    assert_eq!(swapped_amount_b, 526_315_789_473);

    assert_eq!(token_b.balance(&user_a), 1_000_000_000_000);
    assert_eq!(token_a.balance(&user_b), 526_315_789_473);

    let repay_a = contract.repay(&user_a, &token_b.address, &1_000_000_000_000);
    let repay_b = contract.repay(&user_b, &token_a.address, &523_560_209_424);

    assert_eq!(repay_a, (1_000_000_000_000, 1_000_000_000_000));
    assert_eq!(repay_b, (523_560_209_423, 523_560_209_423));

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 523_560_209_424);
    assert_eq!(withdrawn_amount_b, 999_999_999_997);
    assert_eq!(token_a.balance(&user_a), 523_560_209_424);
    assert_eq!(token_b.balance(&user_b), 999_999_999_997);
}
