#![cfg(test)]

use crate::testutils::{register_test_contract, RockPaperScissorsContract};
use crate::token::{self, TokenMetadata};
use rand::{thread_rng, RngCore};
use soroban_auth::{Identifier, Signature};
use soroban_sdk::bytes;
use soroban_sdk::{
    serde::Serialize, testutils::Accounts, AccountId, Address, BigInt, Bytes, BytesN, Env, IntoVal,
};

fn generate_contract_id() -> [u8; 32] {
    let mut id: [u8; 32] = Default::default();
    thread_rng().fill_bytes(&mut id);
    id
}

fn create_token_contract(e: &Env, admin: &AccountId) -> ([u8; 32], token::Client) {
    let id = e.register_contract_token(&BytesN::from_array(
        e,
        &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
    ));
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

fn create_contract(e: &Env) -> ([u8; 32], RockPaperScissorsContract) {
    let id = generate_contract_id();
    register_test_contract(e, &id);
    let contract = RockPaperScissorsContract::new(e, &id);
    contract.initialize();

    (id, contract)
}

#[test]
fn test() {
    let e: Env = Default::default();
    let admin = e.accounts().generate(); // token admin

    // two users for testing
    let user1 = e.accounts().generate();
    let user1_id = Identifier::Account(user1.clone());

    let (_contract1, usdc_token) = create_token_contract(&e, &admin); // registered and initialized the usdc token contract

    let pre_image = Bytes::from_slice(&e, "soroban is awesome".as_bytes());
    let hash = e.compute_hash_sha256(&pre_image);

    let (contract_arr_id, contract) = create_contract(&e);

    let contract_id = Identifier::Contract(BytesN::from_array(&e, &contract_arr_id));
    // the id of the contract

    usdc_token.with_source_account(&admin).mint(
        &Signature::Invoker,
        &BigInt::zero(&e),
        &contract_id,
        &BigInt::from_u32(&e, 1000),
    );

    let user1_address = Address::Account(user1);
    let mut commit_image = Bytes::new(&e);
    match user1_address {
        Address::Account(a) => commit_image.append(&Identifier::Account(a).serialize(&e)),
        _ => panic!("temporary"),
    }

    commit_image.append(&bytes!(&e, 0x526f636b));
    commit_image.append(&Bytes::from_slice(&e, "mysecret".as_bytes()));

    let val = e.compute_hash_sha256(&commit_image);

    contract.commit(&Signature::Invoker, &val);
    let move_pre = contract.check(
        &crate::Player::One,
        &crate::Move::Rock,
        &Bytes::from_slice(&e, "mysecret".as_bytes()),
    );

    assert_eq!(move_pre, bytes!(&e, 0x526f636b));
}
