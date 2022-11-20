#![no_std]

#[cfg(feature = "testutils")]
extern crate std;

mod test;
pub mod testutils;

use soroban_auth::{verify, Identifier, Signature};
use soroban_sdk::{
    bigint, bytes, contracterror, contractimpl, contracttype, panic_with_error, serde::Serialize,
    symbol, BigInt, Bytes, BytesN, Env,
};

mod token {
    soroban_sdk::contractimport!(file = "./soroban_token_spec.wasm");
}

fn check_player(e: &Env, player: Player) -> bool {
    let key = DataKey::Player(player);
    e.data().has(key)
}

fn put_started(e: &Env, started: bool) {
    let key = DataKey::Started;
    e.data().set(key, started);
}

fn game_started(e: &Env) -> bool {
    let key = DataKey::Started;
    e.data().get(key).unwrap_or(Ok(false)).unwrap()
}

fn store_move(e: &Env, player: Player, val: PlayerObj) {
    let key = DataKey::Player(player);
    e.data().set(key, val);
}

fn get_move(e: &Env, player: Player) -> PlayerObj {
    let key = DataKey::Player(player);
    e.data().get(key).unwrap().unwrap()
}

fn put_nonce(e: &Env, id: Identifier) {
    let key = DataKey::Nonce(id.clone());
    e.data().set(key, get_nonce(e, id) + 1);
}

fn get_nonce(e: &Env, id: Identifier) -> BigInt {
    let key = DataKey::Nonce(id);
    e.data()
        .get(key)
        .unwrap_or_else(|| Ok(BigInt::zero(e)))
        .unwrap()
}

fn put_token(e: &Env, token: BytesN<32>) {
    let key = DataKey::Token;
    e.data().set(key, token);
}

fn get_token(e: &Env) -> BytesN<32> {
    let key = DataKey::Token;
    e.data()
        .get(key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn put_bet(e: &Env, amount: BigInt) {
    let key = DataKey::BetAmount;
    e.data().set(key, amount);
}

fn get_bet(e: &Env) -> BigInt {
    let key = DataKey::BetAmount;
    e.data()
        .get(key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn place_bet(e: &Env, from: Identifier) {
    let client = token::Client::new(e, get_token(e));
    client.xfer_from(
        &Signature::Invoker,
        &BigInt::zero(e),
        &from,
        &Identifier::Contract(e.current_contract()),
        &get_bet(e),
    );
}

fn send_profit(e: &Env, to: Identifier, amount: BigInt) {
    let client = token::Client::new(e, get_token(e));
    client.xfer(&Signature::Invoker, &BigInt::zero(e), &to, &amount)
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    GameNotStarted = 1,
    MaxPlayersHit = 2,
    InvalidReveal = 3,
    InvalidOp = 4,
    NotRevealed = 5,
}

#[contracttype]
#[derive(Clone)]
pub enum Player {
    One,
    Two,
}

#[contracttype]
#[derive(Clone)]
pub enum GameResult {
    Winner(Player),
    Draw,
}

#[contracttype]
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Move {
    Rock = 0,
    Paper = 1,
    Scissors = 2,
    Unrevealed = 3,
}

impl Move {
    pub fn as_bytes(&self, env: &Env) -> Bytes {
        match self {
            Move::Rock => bytes!(env, 0x526f636b),
            Move::Paper => bytes!(env, 0x5061706572),
            Move::Scissors => bytes!(env, 0x53636973736f7273),
            _ => panic_with_error!(env, Error::InvalidOp),
        }
    }

    pub fn repr(&self) -> u32 {
        *self as u32
    }
}

#[contracttype]
#[derive(Clone)]
pub struct PlayerObj {
    id: Identifier,
    user_move: BytesN<32>,
    move_pre: Move,
}

impl PlayerObj {
    pub fn new(id: Identifier, user_move: BytesN<32>) -> Self {
        PlayerObj {
            id,
            user_move,
            move_pre: Move::Unrevealed,
        }
    }
}

#[contracttype]
#[derive(Clone)]
/// Contract data keys
pub enum DataKey {
    Started,
    Token,
    BetAmount,
    Nonce(Identifier),
    Player(Player),
}

/// Contract trait
pub trait RockPaperScissorsTrait {
    // leaving this one for possible updates in the future that need a contract initialization
    fn initialize(e: Env, token: BytesN<32>, bet_amount: BigInt) -> Result<(), Error>;

    fn make_move(e: Env, sig: Signature, user_move: BytesN<32>) -> Result<(), Error>;

    fn reveal(e: Env, player: Player, user_move: Move, secret: Bytes) -> Result<Move, Error>;

    fn evaluate(e: Env) -> Result<GameResult, Error>;
}

pub struct RockPaperScissorsContract;

#[contractimpl]
impl RockPaperScissorsTrait for RockPaperScissorsContract {
    fn initialize(e: Env, token: BytesN<32>, bet_amount: BigInt) -> Result<(), Error> {
        if !game_started(&e) {
            put_started(&e, true);
            put_token(&e, token);
            put_bet(&e, bet_amount);
            Ok(())
        } else {
            Err(Error::GameNotStarted)
        }
    }

    fn make_move(e: Env, sig: Signature, user_move: BytesN<32>) -> Result<(), Error> {
        if !game_started(&e) {
            panic!("game started yet")
        }

        let nonce = get_nonce(&e, sig.identifier(&e));
        verify(&e, &sig, symbol!("move"), (&user_move, &nonce));
        put_nonce(&e, sig.identifier(&e)); // putting the nonce even for the Invoker singature

        let player_obj = PlayerObj::new(sig.identifier(&e), user_move);

        if !check_player(&e, Player::One) {
            store_move(&e, Player::One, player_obj);
            place_bet(&e, sig.identifier(&e));
            Ok(())
        } else if !check_player(&e, Player::Two) {
            store_move(&e, Player::Two, player_obj);
            place_bet(&e, sig.identifier(&e));
            Ok(())
        } else {
            Err(Error::MaxPlayersHit)
        }
    }

    // doesn't need authenticating since the revealer needs to know the secret
    // the account id for the hash is only needed so that the hash image doesn't coincide if the same moves are hashed with the same secrets by two different users
    fn reveal(e: Env, player: Player, user_move: Move, secret: Bytes) -> Result<Move, Error> {
        let mut player_obj = get_move(&e, player.clone());

        let mut rhs = Bytes::new(&e);
        rhs.append(&player_obj.clone().id.serialize(&e));
        rhs.append(&user_move.as_bytes(&e));
        rhs.append(&secret);

        let rhs_hash = e.compute_hash_sha256(&rhs);

        if player_obj.user_move == rhs_hash {
            player_obj.move_pre = user_move;
            store_move(&e, player, player_obj);
            Ok(user_move)
        } else {
            Err(Error::InvalidReveal)
        }
    }

    fn evaluate(e: Env) -> Result<GameResult, Error> {
        // check that both players have revealed
        if !check_player(&e, Player::One) || !check_player(&e, Player::Two) {
            return Err(Error::NotRevealed);
        }

        let p1_obj = get_move(&e, Player::One);
        let p2_obj = get_move(&e, Player::Two);

        if (p1_obj.move_pre.repr() + 1) % 3 == p2_obj.move_pre.repr() {
            send_profit(&e, p2_obj.id, get_bet(&e) * bigint!(&e, 2));
            Ok(GameResult::Winner(Player::Two))
        } else if p1_obj.move_pre.repr() == p2_obj.move_pre.repr() {
            // give back the betted money to both players
            send_profit(&e, p1_obj.id, get_bet(&e));
            send_profit(&e, p2_obj.id, get_bet(&e));
            Ok(GameResult::Draw)
        } else {
            send_profit(&e, p1_obj.id, get_bet(&e) * bigint!(&e, 2));
            Ok(GameResult::Winner(Player::One))
        }
    }
}
