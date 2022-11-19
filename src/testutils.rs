#![cfg(any(test, feature = "testutils"))]

use crate::{Move, Player, RockPaperScissorsContractClient};

use soroban_auth::Signature;
use soroban_sdk::{AccountId, Bytes, BytesN, Env};

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

    pub fn initialize(&self) {
        self.client().initialize()
    }

    pub fn commit(&self, sig: &Signature, user_move: &BytesN<32>) {
        self.client().make_move(sig, user_move);
    }

    pub fn check(&self, player: &Player, user_move: &Move, secret: &Bytes) -> Bytes {
        self.client().reveal(player, user_move, secret)
    }
}
