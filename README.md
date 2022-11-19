# Soroban commit-reveal game contract

The commit-reveal scheme is a cryptographic technique which allows a user to make a commitment to keep a certain value hidden from external observers, only to reveal the value later in another invocation.

This algorithm works as follows:
1. user submits the commitment in the form of `sha256(user_address, value, secret)`.
2. user calles the reveal invocation with `(value, secret)` as parameters, which are then used along with `env.invoker()` to re-create the inital commitment (`sha256(invoker, value, secret)`). If the recreated commitment and the initial commitment match, it means that `value` is the one from the initial commitment, i.e the user has revealed the commitment.

This very simple proving scheme is very useful if the contract's workflow needs certain user inputs to remain hidden until a certain event, but is also used to prevent front-running attacks (even though I haven't read much about if Soroban would handle front-running like Ethereum does). 

## This contract: a hash guessing game

This contract is a simple game, where participants have to guess what the pre-image of a certain hash is. Using the commit-reeal scheme here protects the contract from a front-running attack, in fact let's say that user A finds the pre-image of the hash, and simply submits it to the contract, user B could observe the call, submit the same solution with a higher tx fee which will be prioritized if the number of operations in the candidate transaction set is greater than the maximum number of operations for the ledger.

By hiding the solution with the commit-reveal scheme, and revealing it relying on with `env.invoker()`, a front-running attack wouldn't work. 

The contract offers three methods.

#### initialize(e: Env, hash: BytesN<32>)
Starts the game by setting the satus, and putting the image of the hash function:

```rust
fn initialize(e: Env, hash: BytesN<32>) {
        if game_started(&e) {
            panic!("game already started")
        }

        put_hash(&e, hash);
        put_started(&e, true);
    }
```

#### commit(e: Env, val: BytesN<32>)
Used to submit the initial commitment. Simply checks if the game has started (i.e contract initialized), and then puts the commitment in the `DataKey::Commit(Address)` data entry:

```rust
fn commit(e: Env, val: BytesN<32>) {
        if !game_started(&e) {
            panic!("game started yet")
        }

        store_commit(&e, e.invoker(), val);
    }
```

#### check(e: Env, guess: Bytes, secret: Bytes)
This is where most things happen:
1. The contract receives the solution and the secret used in the initial commitment
2. re-creates the initial commitment with the two supplied params and `env.invoker()`
3. matches the re-created commitment against the initial one, if they don't match the contract panics
4. checks that `guess` (the solution param) is the pre-image of the hash with `env.compute_hash_sha256`
5. if everything goes right the contract sends 100 usdc (or at least what the tests believe the usdc contract to be (`[0; 32]`)).

```rust
fn check(e: Env, guess: Bytes, secret: Bytes) {
        let invoker = e.invoker();
        let invoker_id: Identifier;
        let commit = get_commit(&e, invoker.clone());

        let mut rhs = Bytes::new(&e);
        match invoker {
            Address::Account(a) => {
                rhs.append(&a.clone().serialize(&e));
                invoker_id = Identifier::Account(a)
            }
            Address::Contract(a) => {
                rhs.append(&a.clone().into());
                invoker_id = Identifier::Contract(a)
            } // why not support contracts that play the game :-)
        }

        rhs.append(&guess);
        rhs.append(&secret);
        let rhs_commit = e.compute_hash_sha256(&rhs);

        if commit != rhs_commit {
            panic!("params don't match the commitment")
        }

        if e.compute_hash_sha256(&guess) != get_hash(&e) {
            panic!("wrong solution")
        }

        send_reward(&e, invoker_id);
    }
```

## Testing
The tests should be quite straightforward if you've taken a look at soroban before. The test includes three simulations where two should panic since one attempts to front-run and the other holds a wrong solution:

```bash
❯ cargo test                     
    Finished test [unoptimized + debuginfo] target(s) in 0.68s
     Running unittests src/lib.rs (target/debug/deps/soroban_commit_reveal_contract-a5329a726c80ce37)

running 3 tests
test test::test_front_run - should panic ... ok
test test::test_wrong_solution - should panic ... ok
test test::test ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests soroban-commit-reveal-contract

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

# Conclusion
Even if I'm not sure yet how front-running would work on soroban, this is a gret way to get started with such proving schemes, which can come in handy even if the contract isn't trying to protect from fron-running attacks.
