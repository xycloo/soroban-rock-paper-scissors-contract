#![cfg(any(test, feature = "testutils"))]

use crate::{GameResult, Move, Player, RockPaperScissorsContractClient, TimeStamp};

use soroban_auth::Signature;
use soroban_sdk::{BigInt, Bytes, BytesN, Env};

pub fn register_test_contract(e: &Env, contract_id: &[u8; 32]) {
    let contract_id = BytesN::from_array(e, contract_id);
    e.register_contract(&contract_id, crate::RockPaperScissorsContract {});
}

pub struct RockPaperScissorsContract {
    env: Env,
    contract_id: BytesN<32>,
}

impl RockPaperScissorsContract {
    fn client(&self) -> RockPaperScissorsContractClient {
        RockPaperScissorsContractClient::new(&self.env, &self.contract_id)
    }

    pub fn new(env: &Env, contract_id: &[u8; 32]) -> Self {
        Self {
            env: env.clone(),
            contract_id: BytesN::from_array(env, contract_id),
        }
    }

    pub fn initialize(&self, token: &BytesN<32>, bet_amount: &BigInt, ts_diff: &TimeStamp) {
        self.client().initialize(&token, &bet_amount, &ts_diff)
    }

    pub fn make_move(&self, sig: &Signature, user_move: &BytesN<32>) {
        self.client().make_move(sig, user_move);
    }

    pub fn reveal(&self, player: &Player, user_move: &Move, secret: &Bytes) -> Move {
        self.client().reveal(player, user_move, secret)
    }

    pub fn evaluate(&self) -> GameResult {
        self.client().evaluate()
    }

    pub fn cancel(&self, sig: &Signature, player: &Player) {
        self.client().cancel()
    }
}
