#![no_std]

mod test;

use soroban_sdk::{
    bytes, contracterror, contractimpl, contracttype, panic_with_error, serde::Serialize, symbol,
    Address, Bytes, BytesN, Env,
};

mod token {
    soroban_sdk::contractimport!(file = "./soroban_token_spec.wasm");
}

fn check_player(e: &Env, player: Player) -> bool {
    let key = DataKey::Player(player);
    e.storage().has(&key)
}

fn check_revealed(e: &Env, player: Player) -> bool {
    let obj = get_move(e, player);

    !matches!(obj.move_pre, Move::Unrevealed)
}

fn put_started(e: &Env, started: bool) {
    let key = DataKey::Started;
    e.storage().set(&key, &started);
}

fn game_started(e: &Env) -> bool {
    let key = DataKey::Started;
    e.storage().get(&key).unwrap_or(Ok(false)).unwrap()
}

fn remove_player(e: &Env, player: Player) {
    let key = DataKey::Player(player);
    e.storage().remove(&key);
}

fn store_move(e: &Env, player: Player, val: PlayerObj) {
    let key = DataKey::Player(player);
    e.storage().set(&key, &val);
}

fn get_move(e: &Env, player: Player) -> PlayerObj {
    let key = DataKey::Player(player);
    e.storage()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, Error::InvalidOp))
        .unwrap()
}

fn put_token(e: &Env, token: BytesN<32>) {
    let key = DataKey::Token;
    e.storage().set(&key, &token);
}

fn get_token(e: &Env) -> BytesN<32> {
    let key = DataKey::Token;
    e.storage()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn put_ts_limit(e: &Env, ts_diff: TimeStamp) {
    let key = DataKey::TsLimit;
    e.storage().set(&key, &ts_diff);
}

fn get_ts_limit(e: &Env) -> TimeStamp {
    let key = DataKey::TsLimit;
    e.storage()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn put_bet_start(e: &Env, ts: TimeStamp) {
    let key = DataKey::BetStart;
    e.storage().set(&key, &ts);
}

fn get_bet_start(e: &Env) -> TimeStamp {
    let key = DataKey::BetStart;
    e.storage()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn put_bet(e: &Env, amount: i128) {
    let key = DataKey::BetAmount;
    e.storage().set(&key, &amount);
}

fn get_bet(e: &Env) -> i128 {
    let key = DataKey::BetAmount;
    e.storage()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn place_bet(e: &Env, from: Address) {
    let client = token::Client::new(e, &get_token(e));
    client.xfer_from(
        &e.current_contract_address(),
        &from,
        &e.current_contract_address(),
        &get_bet(e),
    );
}

fn send_profit(e: &Env, to: Address, amount: i128) {
    let client = token::Client::new(e, &get_token(e));
    client.xfer(&e.current_contract_address(), &to, &amount)
}

// Perform arithmetic ops on custom types
trait Arithmetic<Rhs = Self> {
    type Output;

    fn add(self, rhs: Rhs) -> Self::Output;

    fn sub(self, rhs: Rhs) -> Self::Output;
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
#[contracttype]
/// Timestamp type to enforce explicitness
pub struct TimeStamp(pub u64);

impl TimeStamp {
    fn current(e: &Env) -> Self {
        Self(e.ledger().timestamp())
    }
}

impl Arithmetic<TimeStamp> for TimeStamp {
    type Output = TimeStamp;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
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
    LimitNotReached = 6,
    InvalidSignature = 7,
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
    id: Address,
    user_move: BytesN<32>,
    move_pre: Move,
}

impl PlayerObj {
    pub fn new(id: Address, user_move: BytesN<32>) -> Self {
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
    BetStart,
    TsLimit,
    Started,
    Token,
    BetAmount,
    Player(Player),
}

/// Contract trait
pub trait RockPaperScissorsTrait {
    // leaving this one for possible updates in the future that need a contract initialization
    fn initialize(
        e: Env,
        token: BytesN<32>,
        bet_amount: i128,
        ts_diff: TimeStamp,
    ) -> Result<(), Error>;

    fn make_move(e: Env, user: Address, user_move: BytesN<32>) -> Result<(), Error>;

    fn reveal(e: Env, player: Player, user_move: Move, secret: Bytes) -> Result<Move, Error>;

    fn evaluate(e: Env) -> Result<GameResult, Error>;

    fn cancel(e: Env) -> Result<(), Error>;
}

pub struct RockPaperScissorsContract;

#[contractimpl]
impl RockPaperScissorsTrait for RockPaperScissorsContract {
    fn initialize(
        e: Env,
        token: BytesN<32>,
        bet_amount: i128,
        ts_diff: TimeStamp,
    ) -> Result<(), Error> {
        if !game_started(&e) {
            put_started(&e, true);
            put_token(&e, token);
            put_bet(&e, bet_amount);
            put_ts_limit(&e, ts_diff);
            Ok(())
        } else {
            Err(Error::GameNotStarted)
        }
    }

    fn make_move(e: Env, user: Address, user_move: BytesN<32>) -> Result<(), Error> {
        if !game_started(&e) {
            panic!("game started yet")
        }

        user.require_auth();

        let player_obj = PlayerObj::new(user.clone(), user_move);

        if !check_player(&e, Player::One) {
            store_move(&e, Player::One, player_obj);
            place_bet(&e, user);
            Ok(())
        } else if !check_player(&e, Player::Two) {
            store_move(&e, Player::Two, player_obj);
            place_bet(&e, user);
            put_bet_start(&e, TimeStamp::current(&e));
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

        let rhs_hash = e.crypto().sha256(&rhs);

        if player_obj.user_move != rhs_hash {
            return Err(Error::InvalidReveal);
        }

        player_obj.move_pre = user_move;
        store_move(&e, player, player_obj);
        Ok(user_move)
    }

    fn evaluate(e: Env) -> Result<GameResult, Error> {
        // check that both players have revealed
        if !check_revealed(&e, Player::One) || !check_revealed(&e, Player::Two) {
            return Err(Error::NotRevealed);
        }

        let p1_obj = get_move(&e, Player::One);
        let p2_obj = get_move(&e, Player::Two);

        if (p1_obj.move_pre.repr() + 1) % 3 == p2_obj.move_pre.repr() {
            send_profit(&e, p2_obj.id, get_bet(&e) * 2);
            remove_player(&e, Player::One);
            remove_player(&e, Player::Two);
            Ok(GameResult::Winner(Player::Two))
        } else if p1_obj.move_pre.repr() == p2_obj.move_pre.repr() {
            // give back the betted money to both players
            send_profit(&e, p1_obj.id, get_bet(&e));
            send_profit(&e, p2_obj.id, get_bet(&e));
            remove_player(&e, Player::One);
            remove_player(&e, Player::Two);
            Ok(GameResult::Draw)
        } else {
            send_profit(&e, p1_obj.id, get_bet(&e) * 2);
            remove_player(&e, Player::One);
            remove_player(&e, Player::Two);
            Ok(GameResult::Winner(Player::One))
        }
    }

    fn cancel(e: Env) -> Result<(), Error> {
        if TimeStamp::current(&e).sub(get_ts_limit(&e)) < get_bet_start(&e) {
            return Err(Error::LimitNotReached);
        }

        let p_obj: PlayerObj;
        if !check_revealed(&e, Player::One) && check_revealed(&e, Player::Two) {
            p_obj = get_move(&e, Player::Two);
        } else if check_revealed(&e, Player::One) && !check_revealed(&e, Player::Two) {
            p_obj = get_move(&e, Player::One);
        } else {
            return Err(Error::LimitNotReached);
        }

        send_profit(&e, p_obj.id, get_bet(&e) * 2);
        remove_player(&e, Player::One);
        remove_player(&e, Player::Two);
        Ok(())
    }
}
