# Rock-Paper-Scissors Betting Soroban Smart Contract

## High-level overview
This contract allows to bet with another user on a rock paper scissors game. This kind of betting/games design is also something we at @xycloo are interested in for future small protocols. 

This contract allows for two users to play rock papers scissors by having them submit their moves hashed according the a [commit-reveal scheme](https://github.com/Xycloo/soroban-commit-reveal-contract). Once both players have sumbitted their move (without revealing it since it's a hash), they can reveal it. Once both moves are revealed, the contract evaluates the winner using a simple modulo algorithm.

There are a couple of other things to keep in mind:
- the contract has to be initialized by a game admin, which specifies some settings.
- if a user doesn't reveal its move after $\Delta t$ (specified upon initialization) since the second player submitted their move, a user can call the `cancel` function, which resets the game and sends all the betted money to the user who revealed its move (since it assuments that the other user won't reveal theirs since they know the other user has already won).

# Writing the contract
> Reading this README assumes that you already have basic soroban knowledge (if you don't, I recommend looking at the soroban docs or at our previous submissions).

## Workflow
![image](https://user-images.githubusercontent.com/70587974/203127840-7bcc14d4-5fb5-40d1-8e4a-b6c95a0e785a.png)
(done with excalidraw)

### Some things to know
In this contract we are going to use:
- the standard token contract.
- custom types (some with their own implementations (`TimeStamp`, `Move`, `PlayerObj`)).
- contract errors (for better failure reports than a string panic). 
- the same principles of the [commit-reveal scheme from one of our previous submissions](https://github.com/Xycloo/soroban-commit-reveal-contract).

### Writing the data helpers
We are going to use some data helper function in our contract invocation to have a more coincise code inside our contract functions:

```rust
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

fn check_revealed(e: &Env, player: Player) -> bool {
    let obj = get_move(e, player);

    !matches!(obj.move_pre, Move::Unrevealed)
}

fn put_started(e: &Env, started: bool) {
    let key = DataKey::Started;
    e.data().set(key, started);
}

fn game_started(e: &Env) -> bool {
    let key = DataKey::Started;
    e.data().get(key).unwrap_or(Ok(false)).unwrap()
}

fn remove_player(e: &Env, player: Player) {
    let key = DataKey::Player(player);
    e.data().remove(key);
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

fn put_ts_limit(e: &Env, ts_diff: TimeStamp) {
    let key = DataKey::TsLimit;
    e.data().set(key, ts_diff);
}

fn get_ts_limit(e: &Env) -> TimeStamp {
    let key = DataKey::TsLimit;
    e.data()
        .get(key)
        .unwrap_or_else(|| panic_with_error!(e, Error::GameNotStarted))
        .unwrap()
}

fn put_bet_start(e: &Env, ts: TimeStamp) {
    let key = DataKey::BetStart;
    e.data().set(key, ts);
}

fn get_bet_start(e: &Env) -> TimeStamp {
    let key = DataKey::BetStart;
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
```

### Placing bets with the standard token contract
To transfer from the user to the contract, we are going to use allowances. Allowances allow the `standard_token.xfer_from()` function to transfer from a specified address to another as long as there is an allowance for that (we could also use the advanced auth and accept a signature to use the `xfer` function).

On the other hand, sending profits simmly requires the contract to transfer to the winning user.

```rust
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

```

### Timestamp type

To better distinguish timestamps, we create a custom type for them, and have them implement addition and subtraction functions:

```rust

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

```

### Errors
Errors in soroban smart contract work similarly to rust's errors. We need to define a `contracterror` type with a u32 representation so that the host can return the errors in the format `ContractError(Error::ThisError as u32)`:

```rust
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
```

You might have seen that we encountered some of these in our data helpers with `panic_with_error`. As we write our contract, it will become clearer what each error means.

### Other contract types
Below we define other contract types. They should be pretty self-explanatory, also, we need the `as_bytes()` method for the `Move` so that it can be used for building the hash (without serializing). `user_move` in `PlayerObj` is the hash of the user's move commitment, which we'll look at later.

```rust
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
```

### Contract data keys

```rust
#[contracttype]
#[derive(Clone)]
/// Contract data keys
pub enum DataKey {
    BetStart,
    TsLimit,
    Started,
    Token,
    BetAmount,
    Nonce(Identifier),
    Player(Player),
}
```


## Contract functions

```rust

/// Contract trait
pub trait RockPaperScissorsTrait {
    // leaving this one for possible updates in the future that need a contract initialization
    fn initialize(
        e: Env,
        token: BytesN<32>,
        bet_amount: BigInt,
        ts_diff: TimeStamp,
    ) -> Result<(), Error>;

    fn make_move(e: Env, sig: Signature, user_move: BytesN<32>) -> Result<(), Error>;

    fn reveal(e: Env, player: Player, user_move: Move, secret: Bytes) -> Result<Move, Error>;

    fn evaluate(e: Env) -> Result<GameResult, Error>;

    fn cancel(e: Env) -> Result<(), Error>;
}

```

### Initialize
This function is needed to set up the contract by providing some important settings:
- `token`: the tokenID the contract will use.
- `bet_amount`: the ammount of `token` that users will have to bet in order to enter the game.
- `ts_diff`: the previously-mentioned $\Delta t$.

```rust
	fn initialize(
        e: Env,
        token: BytesN<32>,
        bet_amount: BigInt,
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
```

### Make move
Users can call this function to enter the game. They will need to provide a signature (to authenticate) and a `user_move`. The function first performs the auth check and increases the nonce, then assings a `Player` to the user (`One` if the user is the first to make the move, `Second` is they are the second, else it panics since it means that there already are two players).

```rust
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
            put_bet_start(&e, TimeStamp::current(&e));
            Ok(())
        } else {
            Err(Error::MaxPlayersHit)
        }
    }
```

### Reveal
Here the user reveals their move to the contract. This happens by re-creating the `user_move` supplied in `make_move()`. It's worth noting that no auth checks are required here since for the invocation to succeed, it still needs the user's secret.


```rust
	fn reveal(e: Env, player: Player, user_move: Move, secret: Bytes) -> Result<Move, Error> {
        let mut player_obj = get_move(&e, player.clone());

        let mut rhs = Bytes::new(&e);
        rhs.append(&player_obj.clone().id.serialize(&e));
        rhs.append(&user_move.as_bytes(&e));
        rhs.append(&secret);

        let rhs_hash = e.compute_hash_sha256(&rhs);

        if player_obj.user_move != rhs_hash {
            return Err(Error::InvalidReveal);
        }

        player_obj.move_pre = user_move;
        store_move(&e, player, player_obj);
        Ok(user_move)
    }

```

### Evaluate
This funciton can be called by anyone, and it consists of two parts:
1. Checking that both users have revealed their moves.
2. Determine the winner using a modulo algorithm. This algorithm works by assigning a u32 value to the `Move` enum, which is why we specified these u32 representations when defining the enum.

```rust
    fn evaluate(e: Env) -> Result<GameResult, Error> {
        // check that both players have revealed
        if !check_revealed(&e, Player::One) || !check_revealed(&e, Player::Two) {
            return Err(Error::NotRevealed);
        }

        let p1_obj = get_move(&e, Player::One);
        let p2_obj = get_move(&e, Player::Two);

        if (p1_obj.move_pre.repr() + 1) % 3 == p2_obj.move_pre.repr() {
            send_profit(&e, p2_obj.id, get_bet(&e) * bigint!(&e, 2));
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
            send_profit(&e, p1_obj.id, get_bet(&e) * bigint!(&e, 2));
            remove_player(&e, Player::One);
            remove_player(&e, Player::Two);
            Ok(GameResult::Winner(Player::One))
        }
    }

```

### Cancel
This function can also be called by anyone. It prevents the problem of users not revealing their moves after seeing the competitor's revealed move. In fact, if $\Delta t$ has passed since the second user has made their move, and one of the two users didn't reveal their move, the user who revealed the move receives the whole betted amount:

```rust
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

        send_profit(&e, p_obj.id, get_bet(&e) * bigint!(&e, 2));
        remove_player(&e, Player::One);
        remove_player(&e, Player::Two);
        Ok(())
    }
```
