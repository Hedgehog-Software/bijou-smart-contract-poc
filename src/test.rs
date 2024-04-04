#![cfg(test)]
extern crate std;

use crate::constants::{
    COLLATERAL_BUFFER, ORACLE_ADDRESS, SCALE, TIME_TO_EXEC, TIME_TO_MATURE, TIME_TO_REPAY,
};
use crate::types::stage::Stage;
use crate::types::user::User;
use crate::types::user_liq_data::UserLiqData;
use crate::types::{position::Position, storage::DataKey};
use crate::SwapClient;

use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{symbol_short, token, Address, Env, String, Vec};
use token::Client as TokenClient;

use self::oracle_mock::Client;

mod oracle_mock {
    soroban_sdk::contractimport!(
        file = "oracle_mock/target/wasm32-unknown-unknown/release/oracle_mock_contract.wasm"
    );
}

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
    oracle_client: Client<'a>,
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

        let oracle_address = Address::from_string(&String::from_str(&e, ORACLE_ADDRESS));
        let _ = &e.register_contract_wasm(Some(&oracle_address), oracle_mock::WASM);
        let oracle_client = oracle_mock::Client::new(&e, &oracle_address);
        oracle_client.set_spot_rate(&SCALE);

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
            oracle_client,
        }
    }

    fn add_time(e: &Env, time: u64) {
        e.ledger().with_mut(|li| {
            li.timestamp += time;
        });
    }

    // fn mint_token(token_admin_client: StellarAssetClient<'a>, to: &Address, amount: i128) {
    //     token_admin_client.mint(&to, &amount);
    // }
}

#[test]
fn test_init() {
    let forward_rate: i128 = 100;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        contract,
        ..
    } = SwapTest::setup();
    let spot_rate = contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &0,
    );
    assert_eq!(spot_rate, SCALE);
}

#[test]
#[should_panic]
fn test_re_init() {
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
        &TIME_TO_MATURE,
    );
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
}

#[test]
fn test_init_pos() {
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
        &TIME_TO_MATURE,
    );
    let amount_position_b = contract.init_pos(&token_admin, &100, &50, &100);
    assert_eq!(amount_position_b, 200);
}

#[test]
fn test_init_pos_odd() {
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
        &TIME_TO_MATURE,
    );
    let amount_position_b = contract.init_pos(&token_admin, &10, &3, &100);
    assert_eq!(amount_position_b, 333);
}

#[test]
#[should_panic]
fn test_init_pos_unauthorized() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        token_a,
        token_b,
        contract,
        user_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&user_a, &100, &50, &100);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    let (amount_a, collateral_a) = contract.deposit(&user_a, &token_a.address, &100, &20);
    let (amount_b, collateral_b) = contract.deposit(&user_b, &token_b.address, &200, &40);

    assert_eq!(amount_a, 100);
    assert_eq!(amount_b, 200);
    assert_eq!(collateral_a, 20);
    assert_eq!(collateral_b, 40);

    assert_eq!(token_a.balance(&user_a), 880);
    assert_eq!(token_b.balance(&user_b), 760);
}

#[test]
#[should_panic]
fn test_deposit_wrong_amount() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &200, &200);
}

#[test]
#[should_panic]
fn test_deposit_all_positions_closed() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    token_admin_client_a.mint(&user_b, &1000);
    contract.init_pos(&token_admin, &1, &1, &100);
    contract.deposit(&user_a, &token_a.address, &100, &40);
    contract.deposit(&user_b, &token_a.address, &100, &40);
}

#[test]
#[should_panic]
fn test_deposit_insuficient_collateral() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    token_admin_client_a.mint(&user_b, &1000);
    contract.init_pos(&token_admin, &1, &1, &100);
    contract.deposit(&user_a, &token_a.address, &100, &10);
}

#[test]
fn test_deposit_high_collateral() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    token_admin_client_a.mint(&user_b, &1000);
    contract.init_pos(&token_admin, &1, &1, &100);
    contract.deposit(&user_a, &token_a.address, &100, &100);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    contract.withdraw(&user_a);
}

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
        &TIME_TO_MATURE,
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
        &TIME_TO_MATURE,
    );
    contract.set_spot(&user_a, &forward_rate);
}

#[test]
#[should_panic]
fn test_deposit_amount_after_near_leg() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &100, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.deposit(&user_a, &token_a.address, &100, &100);
}

#[test]
fn test_deposit_collateral_after_near_leg() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    let (amount_a, collateral_a) = contract.deposit(&user_a, &token_a.address, &0, &50);

    assert_eq!(amount_a, 100);
    assert_eq!(collateral_a, 70);
}

#[test]
fn test_swap() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);

    assert_eq!(token_b.balance(&user_a), 0);
    let swapped_amount = contract.swap(&user_a);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_a), 100);

    assert_eq!(token_a.balance(&user_b), 0);
    let swapped_amount = contract.swap(&user_b);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_a.balance(&user_b), 100);
}

#[test]
fn test_swap_with_order() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let user_c = Address::generate(&e);
    assert_ne!(user_a, user_c);

    token_admin_client_a.mint(&user_c, &1000);

    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_c, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);

    assert_eq!(token_b.balance(&user_a), 0);
    let swapped_amount = contract.swap(&user_a);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_a), 100);

    assert_eq!(token_a.balance(&user_b), 0);
    let swapped_amount = contract.swap(&user_b);
    assert_eq!(swapped_amount, 200);
    assert_eq!(token_a.balance(&user_b), 200);

    let swapped_amount = contract.swap(&user_c);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_c), 100);
}

#[test]
fn test_swap_with_order_not_enough() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let user_c = Address::generate(&e);
    token_admin_client_a.mint(&user_c, &1000);

    contract.init_pos(&token_admin, &100, &100, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_c, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &100, &20);

    SwapTest::add_time(&e, TIME_TO_EXEC);

    assert_eq!(token_b.balance(&user_a), 0);
    let swapped_amount = contract.swap(&user_a);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_b.balance(&user_a), 100);

    assert_eq!(token_a.balance(&user_b), 0);
    let swapped_amount = contract.swap(&user_b);
    assert_eq!(swapped_amount, 100);
    assert_eq!(token_a.balance(&user_b), 100);

    let swapped_amount = contract.swap(&user_c);
    assert_eq!(swapped_amount, 0);
    assert_eq!(token_b.balance(&user_c), 0);
}

#[test]
fn test_reclaim_used_position_and_swap() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    let reclaimed_a = contract.reclaim(&user_a);
    assert_eq!(reclaimed_a, 0);
    assert_eq!(token_a.balance(&user_a), 880);

    let swapped_a = contract.swap(&user_a);
    let swapped_b = contract.swap(&user_b);

    assert_eq!(swapped_a, 100);
    assert_eq!(token_b.balance(&user_a), 100);

    assert_eq!(swapped_b, 100);
    assert_eq!(token_a.balance(&user_b), 100);
}

#[test]
fn test_reclaim_unused_position_and_swap() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    assert_eq!(token_a.balance(&user_a), 640);
    let reclaimed_a = contract.reclaim(&user_a);
    assert_eq!(reclaimed_a, 100);
    assert_eq!(token_a.balance(&user_a), 740);

    let swapped_a = contract.swap(&user_a);
    let swapped_b = contract.swap(&user_b);

    assert_eq!(swapped_a, 200);
    assert_eq!(token_b.balance(&user_a), 200);

    assert_eq!(swapped_b, 200);
    assert_eq!(token_a.balance(&user_b), 200);
}

#[test]
fn test_repay_a() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
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
            reclaimed_amount: 0,
            withdrawn_amount: 0,
            collateral: 20,
            withdrawn_collateral: 0,
            is_liquidated: false,
        }
    );
}

#[test]
fn test_withdraw_a() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_a.balance(&user_a), 880);
    assert_eq!(token_b.balance(&user_a), 0);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let withdrawn_amount = contract.withdraw(&user_a);
    assert_eq!(withdrawn_amount, 100);
    assert_eq!(token_a.balance(&user_a), 980);
}

#[test]
fn test_withdraw_b() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 760);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let withdrawn_amount = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount, 100);
    assert_eq!(token_b.balance(&user_b), 860);
}

#[test]
#[should_panic]
fn test_withdraw_liquidated() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    oracle_client.set_spot_rate(&90_000_000_000_000);
    contract.liquidate(&user_a, &token_admin);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_a);
}

#[test]
#[should_panic]
fn test_reclaim_at_deposit() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    contract.reclaim(&user_b);
}

#[test]
fn test_reclaim_at_swap() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_b);
    contract.swap(&user_a);

    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 860);

    let reclaimed_deposit = contract.reclaim(&user_a);
    assert_eq!(reclaimed_deposit, 0);
}

#[test]
fn test_reclaim_at_repay() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    let swap_amount = contract.swap(&user_b);
    contract.swap(&user_a);
    assert_eq!(swap_amount, 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(token_b.balance(&user_b), 760);

    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 860);
}

#[test]
fn test_reclaim_at_withdraw() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    let swap_amount = contract.swap(&user_b);
    contract.swap(&user_a);
    assert_eq!(swap_amount, 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(token_b.balance(&user_b), 760);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 860);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 960);
}

#[test]
fn test_full_reclaim() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_b,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let user_c = Address::generate(&e);
    assert_ne!(user_a, user_c);
    token_admin_client_b.mint(&user_c, &120);

    contract.init_pos(&token_admin, &10, &10, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &100, &20);
    contract.deposit(&user_c, &token_b.address, &100, &20);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    let swap_amount = contract.swap(&user_b);
    assert_eq!(swap_amount, 100);

    assert_eq!(token_b.balance(&user_c), 0);
    let reclaimed_deposit = contract.reclaim(&user_c);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_c), 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    contract.repay(&user_a, &token_b.address, &100);
    assert_eq!(token_b.balance(&user_b), 880);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    let withdrawn_amount = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount, 100);

    assert_eq!(token_b.balance(&user_b), 980);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_b.balance(&user_b), 980);

    assert_eq!(contract.withdraw(&user_c), 0);
    assert_eq!(token_b.balance(&user_c), 100);
}

#[test]
fn test_multiple_reclaim() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    let swap_amount = contract.swap(&user_b);
    contract.swap(&user_a);
    assert_eq!(swap_amount, 100);

    assert_eq!(token_b.balance(&user_b), 760);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 860);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    contract.repay(&user_a, &token_b.address, &100);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    assert_eq!(token_b.balance(&user_b), 860);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 960);

    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_b.balance(&user_b), 960);

    let reclaimed_collateral = contract.reclaim_col(&user_b);
    assert_eq!(reclaimed_collateral, 40);
    assert_eq!(token_b.balance(&user_b), 1000);

    let reclaimed_deposit = contract.reclaim(&user_a);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_a.balance(&user_a), 880);
}

#[test]
#[should_panic]
fn test_reclaim_for_non_participant() {
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
        &TIME_TO_MATURE,
    );
    let user_c = Address::generate(&e);
    assert_eq!(token_a.balance(&user_c), 0);
    assert_eq!(token_b.balance(&user_c), 0);

    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    SwapTest::add_time(&e, TIME_TO_REPAY);

    contract.reclaim(&user_c);
}

#[test]
fn test_reclaim_collateral() {
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_a);
    assert_eq!(token_a.balance(&user_a), 980);

    let reclaimed_collateral = contract.reclaim_col(&user_a);
    assert_eq!(reclaimed_collateral, 20);
    assert_eq!(token_a.balance(&user_a), 1000);
}

#[test]
fn test_liquidated_reclaim_col() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100);
    contract.deposit(&user_a, &token_a.address, &100, &200);
    contract.deposit(&user_b, &token_b.address, &200, &400);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 2);
    oracle_client.set_spot_rate(&50_000_000_000_000);
    let reclaimed_collateral = contract.reclaim_col(&user_a);
    assert_eq!(reclaimed_collateral, 160);
    assert_eq!(token_a.balance(&user_a), 860);
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
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &100, &800);
    contract.deposit(&user_a, &token_a.address, &800, &200);
    contract.deposit(&user_b, &token_b.address, &800, &200);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 800);
    oracle_client.set_spot_rate(&70_000_000_000_000);
    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 2);
    assert_eq!(token_a.balance(&token_admin), 2);
}

#[test]
fn test_liquidate_reclaimed_collateral() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &100, &800);
    contract.deposit(&user_a, &token_a.address, &800, &200);
    contract.deposit(&user_b, &token_b.address, &800, &200);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    oracle_client.set_spot_rate(&70_000_000_000_000);
    contract.reclaim_col(&user_a);
    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 2);
    assert_eq!(token_a.balance(&token_admin), 2);
}

#[test]
fn test_no_liquidate_swap() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    token_admin_client_a.mint(&user_a, &100);
    contract.init_pos(&token_admin, &100, &100, &800);
    contract.deposit(&user_a, &token_a.address, &800, &200);
    contract.deposit(&user_b, &token_b.address, &800, &200);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 800);
    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 0);
    assert_eq!(token_a.balance(&token_admin), 0);
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
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &10, &10, &800);
    contract.deposit(&user_a, &token_a.address, &800, &200);
    contract.deposit(&user_b, &token_b.address, &800, &200);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &800);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    assert_eq!(token_a.balance(&user_b), 800);
    oracle_client.set_spot_rate(&120_000_000_000_000);
    let reward_amount = contract.liquidate(&user_b, &token_admin);
    assert_eq!(reward_amount, 2);
    assert_eq!(token_b.balance(&token_admin), 2);
}

#[test]
fn test_liquidate_liquidated_user() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &100, &800);
    contract.deposit(&user_a, &token_a.address, &800, &200);
    contract.deposit(&user_b, &token_b.address, &800, &200);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 800);
    oracle_client.set_spot_rate(&70_000_000_000_000);
    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 2);
    assert_eq!(token_a.balance(&token_admin), 2);

    let reward_amount = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount, 0);
    assert_eq!(token_a.balance(&token_admin), 2);
}

#[test]
fn test_forward_bigger_than_spot() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 52_356_020_942_408;
    let forward_rate: i128 = 52_631_578_947_368;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(12_000 * decimals));
    token_admin_client_b.mint(&user_b, &(10_000 * decimals));
    oracle_client.set_spot_rate(&spot_rate);

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let amount_to_deposit_b = contract.init_pos(&token_admin, &100, &100, &(10_000 * decimals));
    let amount_col_b = amount_to_deposit_b * &COLLATERAL_BUFFER / 100;
    assert_eq!(amount_to_deposit_b, 523_560_209_424);

    contract.deposit(
        &user_a,
        &token_a.address,
        &(10_000 * decimals),
        &(2_000 * decimals),
    );
    contract.deposit(
        &user_b,
        &token_b.address,
        &amount_to_deposit_b,
        &amount_col_b,
    );

    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 523_560_209_424);
    assert_eq!(swapped_amount_b, 999_999_999_999);

    assert_eq!(token_b.balance(&user_a), 523_560_209_424);
    assert_eq!(token_a.balance(&user_b), 999_999_999_999);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_b.mint(&user_a, &27_55_580_049);
    let repay_a = contract.repay(&user_a, &token_b.address, &526_315_789_473);
    let repay_b = contract.repay(&user_b, &token_a.address, &999_999_999_999);
    assert_eq!(repay_a, (526_315_789_473, 526_315_789_473));
    assert_eq!(repay_b, (999_999_999_999, 999_999_999_999));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 999_999_999_998);
    assert_eq!(withdrawn_amount_b, 526_315_789_473);
    assert_eq!(token_a.balance(&user_a), 1_000_000_000_998);
    assert_eq!(token_b.balance(&user_b), 898_043_539_165);

    let reclaim_col_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_b, amount_col_b);
    assert_eq!(token_b.balance(&user_b), 1_002_755_581_049);
}

#[test]
fn test_forward_smaller_than_spot() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 52_631_578_947_368;
    let forward_rate: i128 = 52_356_020_942_408;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(12_000 * decimals));
    token_admin_client_b.mint(&user_b, &(10_000 * decimals));

    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("GBP"),
        &symbol_short!("USD"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let amount_deposit_b = contract.init_pos(&token_admin, &100, &100, &(10_000 * decimals));
    assert_eq!(amount_deposit_b, 526_315_789_473);
    let token_b_collateral = 526_315_789_473 * COLLATERAL_BUFFER / 100;

    contract.deposit(
        &user_a,
        &token_a.address,
        &(10_000 * decimals),
        &(2_000 * decimals),
    );
    contract.deposit(
        &user_b,
        &token_b.address,
        &526_315_789_473,
        &token_b_collateral,
    );
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 526_315_789_473);
    assert_eq!(swapped_amount_b, 999_999_999_998);

    assert_eq!(token_b.balance(&user_a), 526_315_789_473);
    assert_eq!(token_a.balance(&user_b), 999_999_999_998);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    let repay_a = contract.repay(&user_a, &token_b.address, &523_560_209_423);
    let repay_b = contract.repay(&user_b, &token_a.address, &999_999_999_998);

    assert_eq!(repay_a, (523_560_209_423, 523_560_209_423));
    assert_eq!(repay_b, (999_999_999_998, 999_999_999_998));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 999_999_999_997);
    assert_eq!(withdrawn_amount_b, 523_560_209_423);
    assert_eq!(token_a.balance(&user_a), 1_000_000_000_997);
    assert_eq!(token_b.balance(&user_b), 891_981_263_056);

    let reclaim_amount_a = contract.reclaim(&user_a);
    assert_eq!(reclaim_amount_a, 0);

    let reclaim_amount_b = contract.reclaim(&user_b);
    assert_eq!(reclaim_amount_b, 0);

    let reclaim_col_amount_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_amount_b, token_b_collateral);
}

#[test]
fn test_liquidate_devaluation() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 190_000_000_000_000;
    let forward_rate: i128 = 191_000_000_000_000;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(20_000 * decimals));
    token_admin_client_b.mint(&user_b, &(26_000 * decimals));

    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_REPAY,
    );
    let deposit_amount_b = contract.init_pos(&token_admin, &100, &100, &(10_000 * decimals));
    assert_eq!(deposit_amount_b, 1_900_000_000_000);
    contract.deposit(
        &user_a,
        &token_a.address,
        &(10_000 * decimals),
        &(2_200 * decimals),
    );
    contract.deposit(
        &user_b,
        &token_b.address,
        &(19_000 * decimals),
        &(4_000 * decimals),
    );
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_900_000_000_000);
    assert_eq!(swapped_amount_b, 1_000_000_000_000);

    let reward_amount_a = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount_a, 0);
    let reward_amount_b = contract.liquidate(&user_b, &token_admin);
    assert_eq!(reward_amount_b, 0);

    oracle_client.set_spot_rate(&380_000_000_000_000);

    let reward_amount_a = contract.liquidate(&user_a, &token_admin);
    assert_eq!(reward_amount_a, 0);
    let reward_amount_b = contract.liquidate(&user_b, &token_admin);
    assert_eq!(reward_amount_b, 4_000_000_000);
}

#[test]
fn test_one_to_many_position() {
    let SwapTest {
        e,
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
    let forward_rate: i128 = SCALE;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(12_000 * decimals));
    token_admin_client_b.mint(&user_b, &(10_000 * decimals));

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let token_b_collateral = 333_333_333_333 * COLLATERAL_BUFFER / 100;
    contract.init_pos(&token_admin, &1, &3, &(10_000 * decimals));
    contract.deposit(
        &user_a,
        &token_a.address,
        &(10_000 * decimals),
        &(2_000 * decimals),
    );
    contract.deposit(
        &user_b,
        &token_b.address,
        &333_333_333_333,
        &token_b_collateral,
    );

    let user_c = Address::generate(&e);
    let user_d = Address::generate(&e);
    token_admin_client_b.mint(&user_c, &(333_333_333_333 + token_b_collateral));
    token_admin_client_b.mint(&user_d, &(333_333_333_333 + token_b_collateral));

    contract.deposit(
        &user_c,
        &token_b.address,
        &333_333_333_333,
        &token_b_collateral,
    );
    contract.deposit(
        &user_d,
        &token_b.address,
        &333_333_333_333,
        &token_b_collateral,
    );

    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    let swapped_amount_c = contract.swap(&user_c);
    let swapped_amount_d = contract.swap(&user_d);
    assert_eq!(swapped_amount_a, 999_999_999_999);
    assert_eq!(swapped_amount_b, 333_333_333_333);
    assert_eq!(swapped_amount_c, 333_333_333_333);
    assert_eq!(swapped_amount_d, 333_333_333_333);

    assert_eq!(token_b.balance(&user_a), 999_999_999_999);
    assert_eq!(token_a.balance(&user_b), 333_333_333_333);
    assert_eq!(token_a.balance(&user_c), 333_333_333_333);
    assert_eq!(token_a.balance(&user_d), 333_333_333_333);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    let repay_a = contract.repay(&user_a, &token_b.address, &999_999_999_999);
    let repay_b = contract.repay(&user_b, &token_a.address, &333_333_333_333);
    let repay_c = contract.repay(&user_c, &token_a.address, &333_333_333_333);
    let repay_d = contract.repay(&user_d, &token_a.address, &333_333_333_333);

    assert_eq!(repay_a, (999_999_999_999, 999_999_999_999));
    assert_eq!(repay_b, (333_333_333_333, 333_333_333_333));
    assert_eq!(repay_c, (333_333_333_333, 333_333_333_333));
    assert_eq!(repay_d, (333_333_333_333, 333_333_333_333));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    let withdrawn_amount_c = contract.withdraw(&user_c);
    let withdrawn_amount_d = contract.withdraw(&user_d);
    assert_eq!(withdrawn_amount_a, 999_999_999_999);
    assert_eq!(withdrawn_amount_b, 333_333_333_333);
    assert_eq!(withdrawn_amount_c, 333_333_333_333);
    assert_eq!(withdrawn_amount_d, 333_333_333_333);
}

#[test]
fn test_stage() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
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
        &TIME_TO_MATURE,
    );
    let stage = contract.stage();
    assert_eq!(stage, Stage::Deposit);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    let stage = contract.stage();
    assert_eq!(stage, Stage::Swap);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    let stage = contract.stage();
    assert_eq!(stage, Stage::Repay);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let stage = contract.stage();
    assert_eq!(stage, Stage::Withdraw);
}

#[test]
fn test_users() {
    let forward_rate: i128 = SCALE;
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USDC"),
        &symbol_short!("EURC"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    let user_c = Address::generate(&e);
    let user_d = Address::generate(&e);
    let user_e = Address::generate(&e);
    token_admin_client_a.mint(&user_c, &1_000);
    token_admin_client_a.mint(&user_d, &1_000);
    token_admin_client_b.mint(&user_e, &1_000);

    contract.init_pos(&token_admin, &100, &50, &100);

    contract.deposit(&user_a, &token_a.address, &100, &20);
    contract.deposit(&user_c, &token_a.address, &100, &20);
    contract.deposit(&user_d, &token_a.address, &100, &20);
    contract.deposit(&user_b, &token_b.address, &200, &40);
    contract.deposit(&user_e, &token_b.address, &200, &40);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    assert_eq!(contract.swap(&user_a), 100);
    assert_eq!(contract.swap(&user_c), 100);
    assert_eq!(contract.swap(&user_d), 100);
    assert_eq!(contract.swap(&user_b), 200);
    assert_eq!(contract.swap(&user_e), 100);

    let (users_a, users_b) = contract.users();
    assert_eq!(
        users_a,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_a.clone(),
                    collateral: 20,
                    min_collateral: 20,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_c.clone(),
                    collateral: 20,
                    min_collateral: 20,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_d.clone(),
                    collateral: 20,
                    min_collateral: 20,
                    is_liquidated: false
                }
            ]
        )
    );

    assert_eq!(
        users_b,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_b.clone(),
                    collateral: 40,
                    min_collateral: 40,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_e.clone(),
                    collateral: 40,
                    min_collateral: 20,
                    is_liquidated: false
                }
            ]
        )
    );

    oracle_client.set_spot_rate(&90_000_000_000_000);
    let (users_a, users_b) = contract.users();
    assert_eq!(
        users_a,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_a.clone(),
                    collateral: 20,
                    min_collateral: 22,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_c.clone(),
                    collateral: 20,
                    min_collateral: 22,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_d.clone(),
                    collateral: 20,
                    min_collateral: 22,
                    is_liquidated: false
                }
            ]
        )
    );

    assert_eq!(
        users_b,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_b.clone(),
                    collateral: 40,
                    min_collateral: 36,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_e.clone(),
                    collateral: 40,
                    min_collateral: 18,
                    is_liquidated: false
                }
            ]
        )
    );

    oracle_client.set_spot_rate(&200_000_000_000_000); // 1 USDC  = 2 EURC
    let (users_a, users_b) = contract.users();
    assert_eq!(
        users_a,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_a.clone(),
                    collateral: 20,
                    min_collateral: 10,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_c.clone(),
                    collateral: 20,
                    min_collateral: 10,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_d.clone(),
                    collateral: 20,
                    min_collateral: 10,
                    is_liquidated: false
                }
            ]
        )
    );

    assert_eq!(
        users_b,
        Vec::from_array(
            &e,
            [
                UserLiqData {
                    address: user_b.clone(),
                    collateral: 40,
                    min_collateral: 80,
                    is_liquidated: false
                },
                UserLiqData {
                    address: user_e.clone(),
                    collateral: 40,
                    min_collateral: 40,
                    is_liquidated: false
                }
            ]
        )
    );
}

#[test]
fn test_e2e_round_values() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 90_000_000_000_000;
    let forward_rate: i128 = 91_000_000_000_000;

    token_admin_client_a.mint(&user_a, &11_999_000);
    token_admin_client_b.mint(&user_b, &10_799_000);
    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &2, &2, &10_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_b, &token_b.address, &9_000_000, &1_800_000);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 9_000_000);
    assert_eq!(swapped_amount_b, 10_000_000);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_b.mint(&user_a, &100_000);
    let repay_a = contract.repay(&user_a, &token_b.address, &9_100_000);
    let repay_b = contract.repay(&user_b, &token_a.address, &10_000_000);

    assert_eq!(repay_a, (9_100_000, 9_100_000));
    assert_eq!(repay_b, (10_000_000, 10_000_000));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 10_000_000);
    assert_eq!(withdrawn_amount_b, 9_100_000);
    assert_eq!(token_a.balance(&user_a), 10_000_000);
    assert_eq!(token_b.balance(&user_b), 9_100_000);

    let reclaim_amount_a = contract.reclaim(&user_a);
    let reclaim_amount_b = contract.reclaim(&user_b);
    assert_eq!(reclaim_amount_a, 0);
    assert_eq!(reclaim_amount_b, 0);
    assert_eq!(token_a.balance(&user_a), 10_000_000);
    assert_eq!(token_b.balance(&user_b), 9_100_000);

    let reclaim_col_a = contract.reclaim_col(&user_a);
    let reclaim_col_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_a, 2_000_000);
    assert_eq!(reclaim_col_b, 1_800_000);

    assert_eq!(token_a.balance(&user_a), 12_000_000);
    assert_eq!(token_b.balance(&user_b), 10_900_000);
}

#[test]
fn test_e2e() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 91_863_245_477_859;
    let forward_rate: i128 = 91_942_156_764_123;
    let user_c = Address::generate(&e);

    token_admin_client_a.mint(&user_a, &11_999_000);
    token_admin_client_b.mint(&user_b, &11_022_589);
    token_admin_client_a.mint(&user_c, &12_000_000);

    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &2, &2, &10_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_b, &token_b.address, &9_186_324, &1_837_265);
    contract.deposit(&user_c, &token_a.address, &10_000_000, &2_000_000);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_c = contract.swap(&user_c);
    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_c, 0);
    assert_eq!(swapped_amount_a, 9_186_324);
    assert_eq!(swapped_amount_b, 9_999_999);

    let reclaim_amount_c = contract.reclaim(&user_c);
    assert_eq!(reclaim_amount_c, 10_000_000);

    let reclaim_col_c = contract.reclaim_col(&user_c);
    assert_eq!(reclaim_col_c, 2_000_000);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_b.mint(&user_a, &7_892);
    let repay_a = contract.repay(&user_a, &token_b.address, &9_194_214);
    let repay_b = contract.repay(&user_b, &token_a.address, &9_999_999);

    assert_eq!(repay_a, (9_194_214, 9_194_214));
    assert_eq!(repay_b, (9_999_999, 9_999_999));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 9_999_998);
    assert_eq!(withdrawn_amount_b, 9_194_214);
    assert_eq!(token_a.balance(&user_a), 9_999_998);
    assert_eq!(token_b.balance(&user_b), 9_194_214);

    let reclaim_amount_a = contract.reclaim(&user_a);
    let reclaim_amount_b = contract.reclaim(&user_b);
    assert_eq!(reclaim_amount_a, 0);
    assert_eq!(reclaim_amount_b, 0);
    assert_eq!(token_a.balance(&user_a), 9_999_998);
    assert_eq!(token_b.balance(&user_b), 9_194_214);

    let reclaim_col_a = contract.reclaim_col(&user_a);
    let reclaim_col_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_a, 2_000_000);
    assert_eq!(reclaim_col_b, 1_837_265);

    assert_eq!(token_a.balance(&user_a), 11_999_998);
    assert_eq!(token_b.balance(&user_b), 11_031_479);
}

#[test]
fn test_e2e_multiple_deposits() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 91_863_245_477_859;
    let forward_rate: i128 = 91_942_156_764_123;

    token_admin_client_a.mint(&user_a, &(11_999_000 + 12_000_000));
    token_admin_client_b.mint(&user_b, &(11_022_589 + 11023589));

    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &2, &2, &10_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_b, &token_b.address, &9_186_324, &1_837_265);
    contract.deposit(&user_b, &token_b.address, &9_186_324, &1_837_265);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 18_372_648);
    assert_eq!(swapped_amount_b, 19_999_998);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_b.mint(&user_a, &15784);
    let repay_a = contract.repay(&user_a, &token_b.address, &18_388_429);
    let repay_b = contract.repay(&user_b, &token_a.address, &19_999_998);

    assert_eq!(repay_a, (18_388_429, 18_388_429));
    assert_eq!(repay_b, (19_999_998, 19_999_998));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 19_999_997);
    assert_eq!(withdrawn_amount_b, 18_388_429);
    assert_eq!(token_a.balance(&user_a), 19_999_997);
    assert_eq!(token_b.balance(&user_b), 18_388_429);

    let reclaim_amount_a = contract.reclaim(&user_a);
    let reclaim_amount_b = contract.reclaim(&user_b);
    assert_eq!(reclaim_amount_a, 0);
    assert_eq!(reclaim_amount_b, 0);
    assert_eq!(token_a.balance(&user_a), 19_999_997);
    assert_eq!(token_b.balance(&user_b), 18_388_429);

    let reclaim_col_a = contract.reclaim_col(&user_a);
    let reclaim_col_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_a, 4_000_000);
    assert_eq!(reclaim_col_b, 3_674_530);

    assert_eq!(token_a.balance(&user_a), 23_999_997);
    assert_eq!(token_b.balance(&user_b), 22_062_959);
}

#[test]
fn test_e2e_multiple_deposits_one_unused_position() {
    let SwapTest {
        e,
        token_admin,
        user_a,
        user_b,
        token_a,
        token_b,
        contract,
        token_admin_client_a,
        token_admin_client_b,
        oracle_client,
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 91_863_245_477_859;
    let forward_rate: i128 = 91_942_156_764_123;

    token_admin_client_a.mint(&user_a, &(11_999_000 + 12_000_000));
    token_admin_client_b.mint(&user_b, &11_022_589);

    oracle_client.set_spot_rate(&spot_rate);
    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("USD"),
        &symbol_short!("GBP"),
        &forward_rate,
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &2, &2, &10_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_a, &token_a.address, &10_000_000, &2_000_000);
    contract.deposit(&user_b, &token_b.address, &9_186_324, &1_837_265);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 9_186_324);
    assert_eq!(swapped_amount_b, 9_999_999);

    let reclaim_amount_a = contract.reclaim(&user_a);
    assert_eq!(reclaim_amount_a, 10_000_000);

    let reclaim_col_a = contract.reclaim_col(&user_a);
    assert_eq!(reclaim_col_a, 1_998_284);    

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_b.mint(&user_a, &7_892);
    let repay_a = contract.repay(&user_a, &token_b.address, &9_194_214);
    let repay_b = contract.repay(&user_b, &token_a.address, &9_999_999);

    assert_eq!(repay_a, (9_194_214, 9_194_214));
    assert_eq!(repay_b, (9_999_999, 9_999_999));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 9_999_998);
    assert_eq!(withdrawn_amount_b, 9_194_214);

    let reclaim_amount_a = contract.reclaim(&user_a);
    let reclaim_amount_b = contract.reclaim(&user_b);
    assert_eq!(reclaim_amount_a, 0);
    assert_eq!(reclaim_amount_b, 0);

    let reclaim_col_a = contract.reclaim_col(&user_a);
    let reclaim_col_b = contract.reclaim_col(&user_b);
    assert_eq!(reclaim_col_a, 2_001_716);
    assert_eq!(reclaim_col_b, 1_837_265);
}

// #[test]
// fn test_multiple_deposits_two_accounts() {
//     let SwapTest {
//         e,
//         token_admin,
//         user_a,
//         user_b,
//         token_a,
//         token_b,
//         contract,
//         token_admin_client_a,
//         token_admin_client_b,
//         ..
//     } = SwapTest::setup();
//     token_admin_client_a.mint(&user_a, &10_000);
//     token_admin_client_b.mint(&user_b, &10_000);
//     let forward_rate: i128 = 80_000_000_000_000;

//     contract.initialize(
//         &token_admin,
//         &token_a.address,
//         &token_b.address,
//         &symbol_short!("GBP"),
//         &symbol_short!("USD"),
//         &forward_rate,
//         &forward_rate,
//         &TIME_TO_MATURE,
//     );
//     contract.init_pos(&token_admin, &10000, &10000, &1, &1);
//     // assert_eq!(e.budget().cpu_instruction_cost(),1249445);
//     // assert_eq!(e.budget().memory_bytes_cost(),190878);
//     for i in 0..100 {
//         e.budget().reset_default();
//         contract.deposit(&user_a, &token_a.address, &1, &1);
//         let cpu = e.budget().cpu_instruction_cost();
//         let mem = e.budget().memory_bytes_cost();
//         std::println!("inst {}, cpu: {}, mem: {}", i, cpu, mem);
//     }
//     // assert_eq!(e.budget().cpu_instruction_cost(),96454169);
//     // assert_eq!(e.budget().memory_bytes_cost(),20252758);

//     let (deposits_a, deposits_b) = contract.deposits();
//     assert_eq!(deposits_a.len(), 100);
//     // assert_eq!(deposits_b.len(), 0);

//     // e.as_contract(&contract.address, || {
//     //     let res = e
//     //         .storage()
//     //         .persistent()
//     //         .get::<DataKey, Vec<Position>>(&DataKey::UsedPositionsA)
//     //         .unwrap();

//     //     assert_eq!(res, Vec::new(&e));
//     // });

//     // let res = e
//     //     .storage()
//     //     .persistent()
//     //     .get::<DataKey, Vec<Position>>(&DataKey::UsedPositionsA)
//     //     .unwrap();

//     // assert_eq!(res, Vec::new(&e));

//     // for _ in 0..49 {
//     //     contract.deposit(&user_b, &token_b.address, &1, &1);
//     // }
//     // contract.deposit(&user_b, &token_b.address, &1, &1);
// }

// #[test]
// fn test_multiple_deposits_multiple_accounts() {
//     let SwapTest {
//         e,
//         token_admin,
//         user_a,
//         user_b,
//         token_a,
//         token_b,
//         contract,
//         token_admin_client_a,
//         token_admin_client_b,
//         ..
//     } = SwapTest::setup();
//     let forward_rate: i128 = 80_000_000_000_000;

//     contract.initialize(
//         &token_admin,
//         &token_a.address,
//         &token_b.address,
//         &symbol_short!("GBP"),
//         &symbol_short!("USD"),
//         &forward_rate,
//         &forward_rate,
//         &TIME_TO_MATURE,
//     );
//     contract.init_pos(&token_admin, &10000, &10000, &1, &1);
//     for _ in 0..27 {
//         let user = Address::generate(&e);
//         token_admin_client_a.mint(&user, &2);
//         contract.deposit(&user, &token_a.address, &1, &1);
//     }

//     for _ in 0..27 {
//         let user = Address::generate(&e);
//         token_admin_client_b.mint(&user, &2);
//         contract.deposit(&user, &token_b.address, &1, &1);
//     }
//     // contract.deposit(&user_b, &token_b.address, &1, &1);
// }
