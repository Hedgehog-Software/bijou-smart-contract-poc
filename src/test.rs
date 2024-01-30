#![cfg(test)]
extern crate std;

use crate::constants::{SCALE, TIME_TO_EXEC, TIME_TO_MATURE, TIME_TO_REPAY};
use crate::types::state::State;
use crate::types::user::User;
use crate::SwapClient;

use soroban_sdk::testutils::{Address as _, Ledger};
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
#[should_panic]
fn test_re_init() {
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
fn test_init_pos() {
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
    contract.init_pos(&token_admin, &100, &50, &100, &100);
}
#[test]
#[should_panic]
fn test_init_pos_panic() {
    let forward_rate: i128 = 100_000_000_000_000;
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
        &0,
    );
    contract.init_pos(&user_a, &100, &50, &100, &100);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
        &0,
    );
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &200, &15);
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
        &0,
    );
    token_admin_client_a.mint(&user_b, &1000);
    contract.init_pos(&token_admin, &1, &1, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &15);
    contract.deposit(&user_b, &token_a.address, &100, &15);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.reclaim(&user_a);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);

    contract.set_spot(&token_admin, &forward_rate);

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
        &0,
    );
    let user_c = Address::generate(&e);
    assert_ne!(user_a, user_c);

    token_admin_client_a.mint(&user_c, &1000);

    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_c, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);

    contract.set_spot(&token_admin, &forward_rate);

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
        &0,
    );
    let user_c = Address::generate(&e);
    token_admin_client_a.mint(&user_c, &1000);

    contract.init_pos(&token_admin, &100, &50, &100, &100);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_c, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &100, &10);

    contract.set_spot(&token_admin, &forward_rate);

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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100, &200);
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
            reclaimed_amount: 0,
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_a.balance(&user_a), 890);
    assert_eq!(token_b.balance(&user_a), 0);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let withdrawn_amount = contract.withdraw(&user_a);
    assert_eq!(withdrawn_amount, 100);
    assert_eq!(token_a.balance(&user_a), 990);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 780);
    SwapTest::add_time(&e, TIME_TO_MATURE);
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    let swap_amount = contract.swap(&user_b);
    assert_eq!(swap_amount, 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 780);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 880);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 980);
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
    token_admin_client_b.mint(&user_c, &110);

    contract.init_pos(&token_admin, &100, &50, &100, &100);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &100, &10);
    contract.deposit(&user_c, &token_b.address, &100, &10);

    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    let swap_amount = contract.swap(&user_b);
    assert_eq!(swap_amount, 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 890);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 990);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_b.balance(&user_b), 990);

    contract.withdraw(&user_c);
    assert_eq!(token_b.balance(&user_c), 0);
    let reclaimed_deposit = contract.reclaim(&user_c);
    assert_eq!(reclaimed_deposit, 100);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    let swap_amount = contract.swap(&user_b);
    assert_eq!(swap_amount, 100);

    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_b, &token_a.address, &100);
    assert_eq!(token_b.balance(&user_b), 780);

    SwapTest::add_time(&e, TIME_TO_REPAY);
    contract.withdraw(&user_b);
    assert_eq!(token_b.balance(&user_b), 880);
    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 100);
    assert_eq!(token_b.balance(&user_b), 980);

    let reclaimed_deposit = contract.reclaim(&user_b);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_b.balance(&user_b), 980);
}

#[test]
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

    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    contract.repay(&user_b, &token_a.address, &100);
    SwapTest::add_time(&e, TIME_TO_REPAY);

    let reclaimed_deposit = contract.reclaim(&user_c);
    assert_eq!(reclaimed_deposit, 0);
    assert_eq!(token_a.balance(&user_c), 0);
    assert_eq!(token_b.balance(&user_c), 0);
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
    contract.init_pos(&token_admin, &100, &50, &100, &200);
    contract.deposit(&user_a, &token_a.address, &100, &10);
    contract.deposit(&user_b, &token_b.address, &200, &20);
    contract.set_spot(&token_admin, &forward_rate);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.swap(&user_a);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &100);
    SwapTest::add_time(&e, TIME_TO_MATURE);
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
        &100,
    );
    contract.init_pos(&token_admin, &100, &50, &900, &900);
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liq_adm(&user_a, &token_admin, &forward_rate);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_a.balance(&token_admin), 1);
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
        &100,
    );
    token_admin_client_a.mint(&user_a, &100);
    contract.init_pos(&token_admin, &100, &50, &900, &900);
    contract.deposit(&user_a, &token_a.address, &900, &200);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liq_adm(&user_a, &token_admin, &forward_rate);
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
    contract.init_pos(&token_admin, &100, &50, &900, &900);
    contract.deposit(&user_a, &token_a.address, &900, &100);
    contract.deposit(&user_b, &token_b.address, &900, &100);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &forward_rate);
    contract.swap(&user_a);
    contract.swap(&user_b);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    contract.repay(&user_a, &token_b.address, &900);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    assert_eq!(token_a.balance(&user_b), 900);
    let reward_amount = contract.liq_adm(&user_b, &token_admin, &forward_rate);
    assert_eq!(reward_amount, 1);
    assert_eq!(token_b.balance(&token_admin), 1);
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
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 191_000_000_000_000;
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
        &TIME_TO_MATURE,
    );
    contract.init_pos(
        &token_admin,
        &100,
        &50,
        &(10_000 * decimals),
        &(10_000 * decimals),
    );
    contract.deposit(&user_a, &token_a.address, &(10_000 * decimals), &1_000);
    contract.deposit(&user_b, &token_b.address, &(10_000 * decimals), &1_000);
    contract.set_spot(&token_admin, &spot_rate);

    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_000_000_000_000);
    assert_eq!(swapped_amount_b, 523_560_209_424);

    assert_eq!(token_b.balance(&user_a), 1_000_000_000_000);
    assert_eq!(token_a.balance(&user_b), 523_560_209_424);

    SwapTest::add_time(&e, TIME_TO_MATURE);

    token_admin_client_a.mint(&user_b, &2755580049);
    let repay_a = contract.repay(&user_a, &token_b.address, &1_000_000_000_000);
    let repay_b = contract.repay(&user_b, &token_a.address, &526_315_789_473);
    assert_eq!(repay_a, (1_000_000_000_000, 1_000_000_000_000));
    assert_eq!(repay_b, (526_315_789_473, 526_315_789_473));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 526_315_789_473);
    assert_eq!(withdrawn_amount_b, 999_999_999_998);
    assert_eq!(token_a.balance(&user_a), 526_315_789_473);
    assert_eq!(token_b.balance(&user_b), 999_999_999_998);
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
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 190_000_000_000_000;
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
        &100,
    );
    contract.init_pos(
        &token_admin,
        &100,
        &50,
        &(10_000 * decimals),
        &(10_000 * decimals),
    );
    contract.deposit(&user_a, &token_a.address, &(10_000 * decimals), &1_000);
    contract.deposit(&user_b, &token_b.address, &(10_000 * decimals), &1_000);
    contract.set_spot(&token_admin, &spot_rate);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_000_000_000_000);
    assert_eq!(swapped_amount_b, 526_315_789_473);

    assert_eq!(token_b.balance(&user_a), 1_000_000_000_000);
    assert_eq!(token_a.balance(&user_b), 526_315_789_473);

    SwapTest::add_time(&e, 100);

    let repay_a = contract.repay(&user_a, &token_b.address, &1_000_000_000_000);
    let repay_b = contract.repay(&user_b, &token_a.address, &523_560_209_424);

    assert_eq!(repay_a, (1_000_000_000_000, 1_000_000_000_000));
    assert_eq!(repay_b, (523_560_209_423, 523_560_209_423));

    SwapTest::add_time(&e, TIME_TO_REPAY);

    let withdrawn_amount_a = contract.withdraw(&user_a);
    let withdrawn_amount_b = contract.withdraw(&user_b);
    assert_eq!(withdrawn_amount_a, 523_560_209_424);
    assert_eq!(withdrawn_amount_b, 999_999_999_997);
    assert_eq!(token_a.balance(&user_a), 523_560_209_424);
    assert_eq!(token_b.balance(&user_b), 999_999_999_997);
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
        ..
    } = SwapTest::setup();
    let spot_rate: i128 = 190_000_000_000_000;
    let forward_rate: i128 = 191_000_000_000_000;
    let decimals = 100_000_000;

    token_admin_client_a.mint(&user_a, &(20_000 * decimals));
    token_admin_client_b.mint(&user_b, &(20_000 * decimals));

    contract.initialize(
        &token_admin,
        &token_a.address,
        &token_b.address,
        &symbol_short!("GBP"),
        &symbol_short!("USD"),
        &forward_rate,
        &TIME_TO_REPAY,
    );
    contract.init_pos(
        &token_admin,
        &100,
        &50,
        &(10_000 * decimals),
        &(10_000 * decimals),
    );
    contract.deposit(&user_a, &token_a.address, &(10_000 * decimals), &(2_000 * decimals));
    contract.deposit(&user_b, &token_b.address, &(10_000 * decimals), &(1_048 * decimals));
    contract.set_spot(&token_admin, &spot_rate);
    SwapTest::add_time(&e, TIME_TO_EXEC);

    let swapped_amount_a = contract.swap(&user_a);
    let swapped_amount_b = contract.swap(&user_b);
    assert_eq!(swapped_amount_a, 1_000_000_000_000);
    assert_eq!(swapped_amount_b, 526_315_789_473);

    let reward_amount = contract.liq_adm(&user_a, &token_admin, &forward_rate);
    assert_eq!(reward_amount, 0);

    let reward_amount = contract.liq_adm(&user_a, &token_admin, &380_000_000_000_000);
    assert_eq!(reward_amount, 2_000_000_000);    
}

#[test]
fn test_state() {
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
    SwapTest::add_time(&e, TIME_TO_EXEC);
    let state = contract.state();
    assert_eq!(state, State::Deposit);
    SwapTest::add_time(&e, TIME_TO_EXEC);
    contract.set_spot(&token_admin, &1000000);
    let state = contract.state();
    assert_eq!(state, State::Swap);
    SwapTest::add_time(&e, TIME_TO_MATURE);
    let state = contract.state();
    assert_eq!(state, State::Repay);
    SwapTest::add_time(&e, TIME_TO_REPAY);
    let state = contract.state();
    assert_eq!(state, State::Withdraw);
}
