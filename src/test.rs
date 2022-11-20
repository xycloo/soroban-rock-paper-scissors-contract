#![cfg(test)]

use crate::testutils::{register_test_contract, RockPaperScissorsContract};
use crate::token::{self, TokenMetadata};
use crate::{GameResult, Move, Player};
use rand::{thread_rng, RngCore};
use soroban_auth::{Identifier, Signature};
use soroban_sdk::bigint;
use soroban_sdk::{
    serde::Serialize, testutils::Accounts, AccountId, BigInt, Bytes, BytesN, Env, IntoVal,
};

fn generate_contract_id() -> [u8; 32] {
    let mut id: [u8; 32] = Default::default();
    thread_rng().fill_bytes(&mut id);
    id
}

fn create_token_contract(e: &Env, admin: &AccountId) -> ([u8; 32], token::Client) {
    let id = e.register_contract_token(&BytesN::from_array(e, &[0; 32]));
    let token = token::Client::new(e, &id);
    // decimals, name, symbol don't matter in tests
    token.init(
        &Identifier::Account(admin.clone()),
        &TokenMetadata {
            name: "USD coin".into_val(e),
            symbol: "USDC".into_val(e),
            decimals: 7,
        },
    );
    (id.into(), token)
}

fn create_contract(
    e: &Env,
    token: BytesN<32>,
    bet_amount: BigInt,
) -> ([u8; 32], RockPaperScissorsContract) {
    let id = generate_contract_id();
    register_test_contract(e, &id);
    let contract = RockPaperScissorsContract::new(e, &id);
    contract.initialize(token, bet_amount);

    (id, contract)
}

#[test]
fn test_rock_wins() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin
    let u1 = e.accounts().generate();

    let (contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let (contract_arr_id, contract) =
        create_contract(&e, BytesN::from_array(&e, &contract1), bigint!(&e, 10));

    let contract_id = Identifier::Contract(BytesN::from_array(&e, &contract_arr_id));
    // the id of the contract

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(admin.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&admin).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(u1.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&u1).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    let mut admin_make_move_image = Bytes::new(&e);
    admin_make_move_image.append(&Identifier::Account(admin.clone()).serialize(&e));
    admin_make_move_image.append(&Move::Rock.as_bytes(&e));
    admin_make_move_image.append(&Bytes::from_slice(&e, "mysecret".as_bytes()));
    let val = e.compute_hash_sha256(&admin_make_move_image);
    e.set_source_account(&admin);
    contract.make_move(&Signature::Invoker, &val);

    let mut u1_make_move_image = Bytes::new(&e);
    u1_make_move_image.append(&Identifier::Account(u1.clone()).serialize(&e));
    u1_make_move_image.append(&Move::Scissors.as_bytes(&e));
    u1_make_move_image.append(&Bytes::from_slice(&e, "u1mysecret".as_bytes()));
    let u1_val = e.compute_hash_sha256(&u1_make_move_image);
    e.set_source_account(&u1);
    contract.make_move(&Signature::Invoker, &u1_val);

    let move_pre = contract.reveal(
        &Player::One,
        &Move::Rock,
        &Bytes::from_slice(&e, "mysecret".as_bytes()),
    );
    matches!(move_pre, Move::Rock);
    let u1_move_pre = contract.reveal(
        &Player::Two,
        &Move::Scissors,
        &Bytes::from_slice(&e, "u1mysecret".as_bytes()),
    );
    matches!(u1_move_pre, Move::Scissors);

    matches!(contract.evaluate(), GameResult::Winner(Player::One));
    assert_eq!(usdc_token.balance(&Identifier::Account(admin)), 20);
    assert_eq!(usdc_token.balance(&Identifier::Account(u1)), 0);
}

#[test]
fn test_draw() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin
    let u1 = e.accounts().generate();

    let (contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let (contract_arr_id, contract) =
        create_contract(&e, BytesN::from_array(&e, &contract1), bigint!(&e, 10));

    let contract_id = Identifier::Contract(BytesN::from_array(&e, &contract_arr_id));
    // the id of the contract

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(admin.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&admin).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(u1.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&u1).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    let mut admin_make_move_image = Bytes::new(&e);
    admin_make_move_image.append(&Identifier::Account(admin.clone()).serialize(&e));
    admin_make_move_image.append(&Move::Rock.as_bytes(&e));
    admin_make_move_image.append(&Bytes::from_slice(&e, "mysecret".as_bytes()));
    let val = e.compute_hash_sha256(&admin_make_move_image);
    e.set_source_account(&admin);
    contract.make_move(&Signature::Invoker, &val);

    let mut u1_make_move_image = Bytes::new(&e);
    u1_make_move_image.append(&Identifier::Account(u1.clone()).serialize(&e));
    u1_make_move_image.append(&Move::Rock.as_bytes(&e));
    u1_make_move_image.append(&Bytes::from_slice(&e, "u1mysecret".as_bytes()));
    let u1_val = e.compute_hash_sha256(&u1_make_move_image);
    e.set_source_account(&u1);
    contract.make_move(&Signature::Invoker, &u1_val);

    let move_pre = contract.reveal(
        &Player::One,
        &Move::Rock,
        &Bytes::from_slice(&e, "mysecret".as_bytes()),
    );
    matches!(move_pre, Move::Rock);
    let u1_move_pre = contract.reveal(
        &Player::Two,
        &Move::Rock,
        &Bytes::from_slice(&e, "u1mysecret".as_bytes()),
    );
    matches!(u1_move_pre, Move::Rock);

    matches!(contract.evaluate(), GameResult::Draw);
    assert_eq!(usdc_token.balance(&Identifier::Account(admin)), 10);
    assert_eq!(usdc_token.balance(&Identifier::Account(u1)), 10);
}

#[test]
fn test_paper_wins() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin
    let u1 = e.accounts().generate();

    let (contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let (contract_arr_id, contract) =
        create_contract(&e, BytesN::from_array(&e, &contract1), bigint!(&e, 10));

    let contract_id = Identifier::Contract(BytesN::from_array(&e, &contract_arr_id));
    // the id of the contract

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(admin.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&admin).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &Identifier::Account(u1.clone()),
        &BigInt::from_u32(&e, 10),
    );

    usdc_token.with_source_account(&u1).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    let mut admin_make_move_image = Bytes::new(&e);
    admin_make_move_image.append(&Identifier::Account(admin.clone()).serialize(&e));
    admin_make_move_image.append(&crate::Move::Paper.as_bytes(&e));
    admin_make_move_image.append(&Bytes::from_slice(&e, "mysecret".as_bytes()));
    let val = e.compute_hash_sha256(&admin_make_move_image);
    e.set_source_account(&admin);
    contract.make_move(&Signature::Invoker, &val);

    let mut u1_make_move_image = Bytes::new(&e);
    u1_make_move_image.append(&Identifier::Account(u1.clone()).serialize(&e));
    u1_make_move_image.append(&crate::Move::Rock.as_bytes(&e));
    u1_make_move_image.append(&Bytes::from_slice(&e, "u1mysecret".as_bytes()));
    let u1_val = e.compute_hash_sha256(&u1_make_move_image);
    e.set_source_account(&u1);
    contract.make_move(&Signature::Invoker, &u1_val);

    let move_pre = contract.reveal(
        &crate::Player::One,
        &crate::Move::Paper,
        &Bytes::from_slice(&e, "mysecret".as_bytes()),
    );
    matches!(move_pre, crate::Move::Paper);

    let u1_move_pre = contract.reveal(
        &crate::Player::Two,
        &crate::Move::Rock,
        &Bytes::from_slice(&e, "u1mysecret".as_bytes()),
    );
    matches!(u1_move_pre, crate::Move::Rock);

    matches!(contract.evaluate(), GameResult::Winner(Player::One));
    assert_eq!(usdc_token.balance(&Identifier::Account(admin)), 20);
    assert_eq!(usdc_token.balance(&Identifier::Account(u1)), 0);
}
