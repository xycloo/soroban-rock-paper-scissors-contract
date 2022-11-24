#![cfg(test)]

use crate::testutils::{register_test_contract, RockPaperScissorsContract};
use crate::token::{self, TokenMetadata};
use crate::{GameResult, Move, Player, TimeStamp};
use rand::{thread_rng, RngCore};
use soroban_auth::{Identifier, Signature};
use soroban_sdk::testutils::{Ledger, LedgerInfo};
use soroban_sdk::{bigint, bytes};
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
    ts_diff: TimeStamp,
) -> ([u8; 32], RockPaperScissorsContract) {
    let id = generate_contract_id();
    register_test_contract(e, &id);
    let contract = RockPaperScissorsContract::new(e, &id);
    contract.initialize(&token, &bet_amount, &ts_diff);

    (id, contract)
}

#[test]
fn test_rock_wins() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin
    let u1 = e.accounts().generate();

    let (contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let (contract_arr_id, contract) = create_contract(
        &e,
        BytesN::from_array(&e, &contract1),
        bigint!(&e, 10),
        TimeStamp(3600),
    );

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

    let (contract_arr_id, contract) = create_contract(
        &e,
        BytesN::from_array(&e, &contract1),
        bigint!(&e, 10),
        TimeStamp(3600),
    );

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

    let (contract_arr_id, contract) = create_contract(
        &e,
        BytesN::from_array(&e, &contract1),
        bigint!(&e, 10),
        TimeStamp(3600),
    );

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

#[test]
fn test_cancel_and_replay() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin
    let u1 = e.accounts().generate();

    extern crate std;
    std::println!(
        "{:?} {:?}",
        admin,
        Identifier::Account(admin.clone()).serialize(&e)
    );

    e.ledger().set(LedgerInfo {
        timestamp: 1666359075,
        protocol_version: 1,
        sequence_number: 10,
        network_passphrase: Default::default(),
        base_reserve: 10,
    });

    let (contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let (contract_arr_id, contract) = create_contract(
        &e,
        BytesN::from_array(&e, &contract1),
        bigint!(&e, 10),
        TimeStamp(3600),
    );

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

    e.ledger().set(LedgerInfo {
        timestamp: 1666362676,
        protocol_version: 1,
        sequence_number: 10,
        network_passphrase: Default::default(),
        base_reserve: 10,
    });
    // u1 hasn't revealed yet one hour after making the bet
    e.set_source_account(&admin);
    contract.cancel(&Signature::Invoker, &Player::One);

    assert_eq!(usdc_token.balance(&Identifier::Account(admin.clone())), 20);
    assert_eq!(usdc_token.balance(&Identifier::Account(u1.clone())), 0);
    assert_eq!(usdc_token.balance(&contract_id), 0);

    usdc_token.with_source_account(&admin).approve(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &bigint!(&e, 10),
    );

    e.set_source_account(&admin);
    contract.make_move(&Signature::Invoker, &val);

    let move_pre = contract.reveal(
        &crate::Player::One,
        &crate::Move::Paper,
        &Bytes::from_slice(&e, "mysecret".as_bytes()),
    );
    matches!(move_pre, crate::Move::Paper);

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
    e.set_source_account(&u1);
    contract.make_move(&Signature::Invoker, &u1_val);
    let u1_move_pre = contract.reveal(
        &crate::Player::Two,
        &crate::Move::Rock,
        &Bytes::from_slice(&e, "u1mysecret".as_bytes()),
    );
    matches!(u1_move_pre, crate::Move::Rock);

    matches!(contract.evaluate(), GameResult::Winner(Player::One));
    assert_eq!(usdc_token.balance(&Identifier::Account(admin)), 30);
    assert_eq!(usdc_token.balance(&Identifier::Account(u1)), 0);
}

#[test]
fn test_build_hash() {
    extern crate std;
    extern crate hex;

    let e: Env = Default::default();

    e.ledger().set(LedgerInfo {
        timestamp: 1668106305,
        protocol_version: 20,
        sequence_number: 10,
        network_passphrase: "Test SDF Future Network ; October 2022".as_bytes().to_vec(),
        base_reserve: 10,
    });

    extern crate stellar_strkey;
    let public = "GBZSAPPCSJC7UQNABF7C7PJZSW2S2H3BTKTVWEXB53WPPA6PXP6AYZ62";
    let decoded = stellar_strkey::StrkeyPublicKeyEd25519::from_string(&public)
        .unwrap()
        .0;

    let mut serialized_bytes = Bytes::from_array(
        &e,
        &[
            0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 7, 65, 99, 99,
            111, 117, 110, 116, 0, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 0,
        ],
    );

    serialized_bytes.append(&Bytes::from_array(&e, &decoded));

    let mut admin_make_move_image = Bytes::new(&e);
    admin_make_move_image.append(&serialized_bytes);
    admin_make_move_image.append(&Move::Scissors.as_bytes(&e));
    admin_make_move_image.append(&Bytes::from_slice(&e, "mysecret1".as_bytes()));
    let val = e.compute_hash_sha256(&admin_make_move_image);

    std::println!("{:?}", hex::encode(val.to_array()));
}
