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

# Futurenet
> Why not play the game on futurenet?

In order to play the game on futurenet you'll need:
1. A token contract with at least two users that have a balance (can be any token).
2. A rock-paper-scissors contract (let's call it RPSC) deployed, and initialized.
3. An allowance to allow the RPSC to spend `n` of `$TOKEN` where `n` and `$TOKEN` are defined when initializing the RPSC.
4. The hash of your move + secret.

Once you have all the above, you are ready to play and bet on a rock paper scissors game on Soroban!

### Having the token contract
I won't stay on this topic much since it's not in the scope of this README, but can follow these steps:
1. create a stellar classic asset, wrap it using the CLI or the examples in the soroban branch of the Python-stellar SDK (https://github.com/StellarCN/py-stellar-base/blob/soroban/examples/soroban_deploy_create_wrapped_token_contract.py ).
2. make sure that you have an account that holds a certain amount of those tokens, and then import them using the `import` fn of the token contract (remember that you have to think in stroops).
3. You can make sure that the user has imported the balance by using the `balance` function of the token contract.

Example:

```bash
~/Desktop/soroban-classic-wrapping ❯ python3 create.py
simulated transaction: footprint='AAAAAAAAAAMAAAAG03yUs1elnu8w4XZGY/fEp9zBG6YX3YR0xj616FY0t4kAAAADAAAAAwAAAAbTfJSzV6We7zDhdkZj98Sn3MEbphfdhHTGPrXoVjS3iQAAAAQAAAABAAAAAAAAAAEAAAAFAAAABUFkbWluAAAAAAAABtN8lLNXpZ7vMOF2RmP3xKfcwRumF92EdMY+tehWNLeJAAAABAAAAAEAAAAAAAAAAQAAAAUAAAAITWV0YWRhdGE=' cost=Cost(cpu_insns='99383', mem_bytes='19036') results=[TransactionStatusResult(xdr='AAAABAAAAAEAAAAEAAAAINN8lLNXpZ7vMOF2RmP3xKfcwRumF92EdMY+tehWNLeJ')] error=None latest_ledger=937040
setting footprint and signing transaction...
sent transaction: id='4973e0b56a27a9063b1b43c6d007920de85205aa15d730b482ba015d1baab2fe' status=<TransactionStatus.PENDING: 'pending'>
waiting for transaction to be confirmed...
waiting for transaction to be confirmed...
transaction status: id='4973e0b56a27a9063b1b43c6d007920de85205aa15d730b482ba015d1baab2fe' status=<TransactionStatus.SUCCESS: 'success'> results=[TransactionStatusResult(xdr='AAAABAAAAAEAAAAEAAAAINN8lLNXpZ7vMOF2RmP3xKfcwRumF92EdMY+tehWNLeJ')]
contract id: d37c94b357a59eef30e1764663f7c4a7dcc11ba617dd8474c63eb5e85634b789


~/Desktop/soroban-rock-paper-scissors-contract main !2 ❯ soroban invoke \
  --id d37c94b357a59eef30e1764663f7c4a7dcc11ba617dd8474c63eb5e85634b789 \
  --secret-key $MY_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn import \
  --arg '{"object":{"vec":[{"symbol":"Invoker"}]}}' --arg 0 --arg 50000000
success
null
```

### Deploying and initializing the contract
To deploy the contract, you first have to compile the code:

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ cargo +nightly build \
    --target wasm32-unknown-unknown \
    --release \
    -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort
   Compiling soroban-rock-paper-scissors-contract v0.0.0 (/home/tommasodeponti/Desktop/soroban-rock-paper-scissors-contract)
    Finished release [optimized] target(s) in 0.67s
```

Then you can deploy:

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban deploy \
    --wasm target/wasm32-unknown-unknown/release/soroban_rock_paper_scissors_contract.wasm  --secret-key $SECRET --rpc-url  https://future.stellar.kai.run:443/soroban/rpc --network-passphrase 'Test SDF Future Network ; October 2022'
success
7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b
```

Now we have to initialize the contract:

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \
  --secret-key $SOME_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn initialize \
  --arg 'd37c94b357a59eef30e1764663f7c4a7dcc11ba617dd8474c63eb5e85634b789' --arg 10000000 --arg '{"object":{"vec":[{"object":{"u64":3600}}]}}'
```

As you can see, we supplied three arguments: the token contract, the bet amount (in stroops), and the timestamp difference, which is in our case an hour (3600 seconds) (see previous sections to learn about these three parameters).

### The allowance
We approve the previously deployed contract to spend 10000000 stroops of the previously wrapped and imported token:
```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke \                                                                                                                    
  --id d37c94b357a59eef30e1764663f7c4a7dcc11ba617dd8474c63eb5e85634b789 \
  --secret-key $SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn approve \
  --arg '{"object":{"vec":[{"symbol":"Invoker"}]}}' --arg 0 --arg '{"object":{"vec":[{"symbol":"Contract"},{"object":{"bytes":"7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b"}}]}}' --arg 10000000
success
null
```

### Building the hash of the move
Since users don't play in real time one against the other, the contract uses a commitment technique as previously disussed. But how do we generate the hash for a futurenet account?

You can simply use this test function we created:

```rust
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
```

As you can see, you just need to put in your public key, then your move (in this case `Move::Scissors` as bytes), and then a secret which you should not share until you reveal your move.

This will return you a hash in hex format.

## Playing
Now that you have everything set up (remember to repeat steps 3 and 4 (allowance and buillding move hash) for another user), you can invoke the `make_move` fn of the contract:

### Making the move

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \  
  --secret-key $U1_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn make_move \
  --arg '{"object":{"vec":[{"symbol":"Invoker"}]}}' --arg "$U1_MOVE_HASH"
success
null
```

Then you'll have to wait until a second player makes the move:

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \  
  --secret-key $U2_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn make_move \
  --arg '{"object":{"vec":[{"symbol":"Invoker"}]}}' --arg "$U2_MOVE_HASH"
success
null
```

### Revealing
Now each player (u1 as player one since they made the move first, and u2 as player two) has to reveal their move so that the contract can evaluate who the winner is and send them the rewards.

U1 reveal (remember that the invoker here doesn't matter, there just needs to be the secret as hex):

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke \
  --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \
  --secret-key $SOME_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn reveal \
  --arg '{"object":{"vec":[{"symbol":"One"}]}}' --arg '{"u32":0}' --arg "6d79736563726574"

success
0
```

Note that we are invoking for `Player::One` (U1), that we are passing `Move::Rock = 0` as a u32 object, and that the last parameter is the hex encoding of U1's secret (`"mysecret"`).

Now invoking for U2 (`Player::Two`) with their move (`Move::Scissors`), and their secret as hex (`"mysecret1"`):

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke \
  --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \
  --secret-key $SOME_SECRET \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn reveal \
  --arg '{"object":{"vec":[{"symbol":"Two"}]}}' --arg '{"u32":2}' --arg "6d7973656372657431"

success
2
```

You can see that both these function if all the provided parameters are correct will return the u32 representation of the `Move` enum:

```rust
#[contracttype]
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Move {
    Rock = 0,
    Paper = 1,
    Scissors = 2,
    Unrevealed = 3,
}
```

Now that both players have revealed we can call the `evaluate` fn to evaluate the winner and have they receive the bet profit:

```bash
~/Desktop/soroban-rock-paper-scissors-contract main !3 ❯ soroban invoke \
  --id 7236cd5d2607a1a9a3950942a66c2b229afbea5ce2d29714af75f18bf993cb7b \
  --secret-key SB3YZR6KMXEOEWAMS4HUQX4JTWW6METEP3LAXSH2F3GQQT4LOYCR3A44 \
  --rpc-url https://future.stellar.kai.run:443/soroban/rpc \
  --network-passphrase 'Test SDF Future Network ; October 2022' \
  --fn evaluate
success
["Winner",["One"]]
```

You can see that the winner is indeed U1 who played `Move::Rock` against `Move::Scissors`. U1 now has `10` `$TOKEN` more and U2 `10` `$TOKEN` less!

