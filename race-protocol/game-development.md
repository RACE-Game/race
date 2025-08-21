# 🎲 Game Development

Developing games on RACE Protocol revolves around creating a core piece of logic called a **Game Handler**. This handler is written in Rust, compiled to WebAssembly (WASM), and acts as the definitive state machine for your game. The protocol is designed so you can focus purely on your game's rules and state, while the Transactor network handles the complexities of blockchain interaction, networking, and security.

This section will guide you through the process of building, testing, and understanding the core components of a RACE game.

## **Getting Started: Your First Game Handler**

Every game on RACE is a Rust library project that implements the `GameHandler` trait.

### **1. Project Setup**

First, create a new Rust library. This library will contain your game's logic.

```sh
cargo new my_awesome_game --lib
```

### **2. Configure `Cargo.toml`**

Your game needs to be compiled as a `cdylib` (a dynamic library format suitable for WASM) and an `rlib` (for integration testing). You'll also need to add the necessary RACE crates as dependencies.

```toml
# In your game's Cargo.toml

[package]
name = "my_awesome_game"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# The core API for building a game handler
race-api = { workspace = true }
# The procedural macro for WASM boilerplate
race-proc-macro = { workspace = true }
# For serializing and deserializing your game state
borsh = { workspace = true }

# Add serde if you plan to use JSON for custom events
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

[dev-dependencies]
# The race-test crate is essential for testing
race-test = { workspace = true }
anyhow = { workspace = true }
```

### **3. The Basic Structure**

Your game logic will live in `src/lib.rs.` The core structure consists of:

1. A struct that holds your game's state, marked with `#[game_handler]`.
2. An implementation of the `GameHandler` trait for that struct.

Here is a minimal, complete example based on `examples/minimal`:

```rust
// src/lib.rs
use race_api::prelude::*;
use race_proc_macro::game_handler;

// Define any on-chain data used for initialization.
// This is optional if your game has no initial configuration.
#[derive(BorshSerialize, BorshDeserialize)]
struct MinimalAccountData {
    init_n: u64,
}

// The struct holds all the state for your game.
// It must be serializable with Borsh.
// The #[game_handler] macro generates the necessary WASM entry points.
#[derive(BorshSerialize, BorshDeserialize)]
#[game_handler]
struct MyAwesomeGame {
    n: u64,
}

// Implement the main logic of your game here.
impl GameHandler for MyAwesomeGame {
    // This is called once when the game is first loaded.
    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self> {
        // Deserialize the initial configuration data from the on-chain account.
        let account_data: MinimalAccountData = init_account.data()?;
        Ok(Self {
            n: account_data.init_n,
        })
    }

    // This function is called for every event in the game.
    fn handle_event(&mut self, _effect: &mut Effect, event: Event) -> HandleResult<()> {
        match event {
            Event::GameStart => {
                // Game logic for when the game starts.
            }
            // ... handle other events
            _ => {}
        }
        Ok(())
    }

    // This is called just before a checkpoint to record player balances.
    fn balances(&self) -> Vec<PlayerBalance> {
        // For this minimal example, we are not tracking balances.
        vec![]
    }
}
```

## **Implementing the `GameHandler` Trait**

Your game's entire logic is contained within the implementation of the `GameHandler` trait. This trait is the contract between your game and the RACE runtime.

The canonical definition is found in `race-repo/api/src/engine.rs`:

```rust
pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;

    /// Report the balances of players.
    /// The return must contain all players and zero balance is allowed.
    fn balances(&self) -> Vec<PlayerBalance>;
}
```

### **State Management**

The struct that implements `GameHandler` **is your game's state**. It must derive `BorshSerialize` and `BorshDeserialize` so the runtime can save and load it between event executions.

```rust
#[derive(BorshSerialize, BorshDeserialize)]
#[game_handler]
struct PokerGame {
    players: BTreeMap<u64, Player>,
    pot: u64,
    current_stage: Stage,
    // ... other state fields
}
```

**`init_state`**

This function is called once when a game room is first loaded by the Transactor. It initializes your game's state from data stored in the on-chain `GameAccount`.

```rust
// From examples/draw-card/src/lib.rs

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountData {
    pub blind_bet: u64,
    pub min_bet: u64,
    pub max_bet: u64,
}

impl GameHandler for DrawCard {
    fn init_state(init_account: InitAccount) -> HandleResult<Self> {
        // The `data` field from the on-chain account is deserialized.
        let account_data: AccountData = init_account.data()?;

        Ok(Self {
            // Initialize the handler's state from the account data
            min_bet: account_data.min_bet,
            max_bet: account_data.max_bet,
            blind_bet: account_data.blind_bet,
            // ... initialize other fields
            ..Default::default()
        })
    }
    // ...
}
```

**`handle_event`**

This is the main entry point for your game's logic. The runtime calls this function for every event that occurs. Your job is to update your state struct based on the event and use the `effect` object to request any side effects.

```rust
// A typical handle_event implementation structure
fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
    match event {
        // A player-sent custom action
        Event::Custom { sender, raw } => {
            let custom_event = MyGameEvent::try_parse(&raw)?;
            self.handle_custom_event(effect, sender, custom_event)?;
        }
        // A new player has joined and been confirmed
        Event::Join { players } => {
            for p in players {
                self.players.push(Player::from(p));
            }
            if self.players.len() >= 2 {
                effect.start_game();
            }
        }
        // The game is ready to begin
        Event::GameStart => {
            // ... setup a new round ...
        }
        // Random numbers have been generated and can now be used
        Event::SecretsReady => {
            // ... reveal cards, determine winner, etc. ...
        }
        // A player's action timer has run out
        Event::ActionTimeout { player_id } => {
            // ... fold the player, advance the turn, etc. ...
        }
        _ => { /* Ignore events not relevant to this game */ }
    }
    Ok(())
}
```

**`balances`**

This function is called by the runtime just before a checkpoint is created. It must return the current in-game chip/token balance of **every player** involved in the game. This is crucial for the settlement process to correctly calculate payouts and verify the integrity of the game's economy.

```rust
// From examples/raffle/src/lib.rs
fn balances(&self) -> Vec<PlayerBalance> {
    self.players.iter().map(|p| PlayerBalance::new(p.id, p.balance)).collect()
}
```

## **Using Effects for Game Actions**

The `Effect` object is your sole gateway to the world outside your pure WASM logic. You call its methods to describe the actions you want the runtime to perform. The runtime then executes these actions on your behalf.

**Randomness**

RACE uses a multi-party computation (MPC) protocol for fair and verifiable randomness.

* `effect.init_random_state(spec: RandomSpec) -> RandomId`: Requests a new source of randomness. `RandomSpec` can be a `ShuffledList` (like a deck of cards) or a Lottery. This returns a `RandomId` you must store in your state.
* `effect.assign(random_id, player_id, indices)`: Assigns a secret random value (e.g., a card) to a specific player. Only that player will be able to decrypt it.
* `effect.reveal(random_id, indices)`: Reveals a random value to everyone (e.g., community cards).
* `effect.get_revealed(random_id) -> HandleResult<&HashMap<usize, String>>`: After an `Event::SecretsReady` is received, this function lets you access the revealed values.

**Player Decisions**

For actions that must be committed secretly (like choosing Rock, Paper, or Scissors), use the decision mechanism.

* `effect.ask(player_id) -> DecisionId`: Asks a player to make a hidden, binding decision. Store the returned `DecisionId`.
* `effect.release(decision_id)`: Requests the reveal of a previously made decision.
* `effect.get_answer(decision_id) -> HandleResult<&str>`: After an `Event::SecretsReady`, use this to get the plaintext answer for a decision.

**Timeouts**

* `effect.action_timeout(player_id, duration_ms)`: Starts a timer for a specific player. If they don't act in time, an `Event::ActionTimeout` will be sent to your handler.
* `effect.wait_timeout(duration_ms)`: Starts a general-purpose timer. When it expires, an `Event::WaitingTimeout` is sent.

**Settlements & Player Management**

These actions are used to manage player funds and status. Calling any of these methods implicitly marks the current state as a **checkpoint**, preparing it for an on-chain settlement transaction.

* `effect.withdraw(player_id, amount)`: A player cashes out `amount` from their in-game balance to their wallet.
* `effect.eject(player_id)`: Removes a player from the game. Their balance must be handled via `withdraw`.
* `effect.transfer(amount)`: Transfers amount from the game's collective pot to the designated on-chain recipient account (e.g., for rake or commissions).
* `effect.award(player_id, bonus_identifier)`: Awards a player a specific bonus (e.g., an NFT prize) identified by a string.
* `effect.accept_deposit(deposit)` / `effect.reject_deposit(deposit)`: Handles in-game deposits made by players after they've already joined.

**Example: Ending a Poker Hand**

```rust
fn handle_winner(&mut self, effect: &mut Effect, winner_id: u64, loser_id: u64, pot: u64) {
    // The winner receives the pot.
    effect.withdraw(winner_id, pot);
    
    // Both players are ejected to start a new game.
    effect.eject(winner_id);
    effect.eject(loser_id);
    
    // The state is now marked for checkpointing and settlement.
    effect.checkpoint();
}
```

**Logging**

You can print logs from your game handler for debugging. These will appear in the Transactor's logs.

* `effect.info("message")`, `effect.warn("message")`, `effect.error("message")`

***

## Defining Custom Events

To handle player actions, you define your own event enum and implement the CustomEvent trait.

```rust
// From examples/draw-card/src/lib.rs

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameEvent {
    Bet(u64),
    Call,
    Fold,
}

// This allows the event to be serialized and deserialized
// from the `raw` field of an `Event::Custom`.
impl CustomEvent for GameEvent {}

// In your handle_event function:
fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
    match event {
        Event::Custom { sender, raw } => {
            // Safely parse the raw bytes into your specific event type
            let event = GameEvent::try_parse(&raw)?;
            self.custom_handle_event(effect, sender, event)?;
        }
        // ...
    }
    // ...
}
```

***

## Testing Your Game

The `race-test` crate is indispensable for writing correct and secure game logic. It provides helpers to simulate the entire game lifecycle without needing a live blockchain or Transactor.

* **`TestClient`**: Simulates a player or a server node. It manages its own secrets for randomization and decisions.
* **`TestContextBuilder`**: A convenient way to set up a `GameAccount` with players, servers, and initial on-chain data for your tests.
* **`TestHandler`**: A wrapper around your `GameHandler` that simulates the runtime, handling the back-and-forth of system events generated by randomization and other effects.

**Integration Test Example Flow:**

The integration test for the `draw-card` example (`examples/draw-card/src/integration_test.rs`) is the best reference. Here's a conceptual overview:

1. **Setup**: Create `TestClient` instances for each player and a transactor. Use `TestContextBuilder` to create an initial `GameContext` and `TestHandler`.
2. **Player Joins**: Simulate a player joining by creating a Join event and passing it to the handler.
3. **Start Game**: When enough players join, the handler will request to `start_game`. You then call `handle_dispatch_event` on the `TestHandler` to process the resulting `Event::GameStart`.
4. **Simulate The Loop**: The most powerful feature is `handle_until_no_events`. This function simulates the entire back-and-forth between the game handler and the clients for complex processes like randomization.
   * You send one event (e.g., a player's bet).
   * The handler processes it and requests an effect (e.g., `reveal`).
   * The test harness notifies all `TestClients` of the state change.
   * The `TestClients` automatically generate the necessary response events (e.g., `ShareSecrets`).
   * The handler processes these system events, and the loop continues until the game is waiting for the next player action.

```rust
// Simplified example of an integration test
#[test]
fn test_full_game() -> anyhow::Result<()> {
    // 1. Setup clients and context
    let mut alice = TestClient::player("Alice");
    let mut transactor = TestClient::transactor("Transactor");
    let (mut ctx, _) = TestContextBuilder::default()
        .set_transactor(&mut transactor)
        .add_player(&mut alice, 10000)
        .build_with_init_state::<MyGame>()?;

    // 2. Start the game (e.g., after another player joins)
    let join_event = ctx.join(&mut bob, 10000);
    ctx.handle_event(&join_event)?;
    ctx.handle_dispatch_event()?; // This handles the dispatched GameStart

    // 3. A player makes a move
    let bet_event = alice.custom_event(MyGameEvent::Bet(100));

    // 4. Let the test harness handle all resulting system events automatically
    ctx.handle_event_until_no_events(&bet_event, vec![&mut alice, &mut bob, &mut transactor])?;

    // Now, the game state should be waiting for the next player's action.
    // You can assert the state is correct.
    let state = ctx.state();
    assert_eq!(state.pot, 100);

    Ok(())
}
```

## **Testing Your Game**

The `race-test` crate is indispensable for writing correct and secure game logic. It provides helpers to simulate the entire game lifecycle without needing a live blockchain or Transactor.

### **Key Components of the Test Kit**

* **`TestClient`**: Simulates a player or a server node (`transactor`, `validator`). It manages its own secrets for randomization and decisions, mimicking the behavior of a real client.
* **`TestContextBuilder`**: A convenient builder for setting up a mock `GameAccount` with players, servers, and initial on-chain data for your tests.
* **`TestHandler`**: A wrapper around your `GameHandler` that simulates the RACE runtime. It processes events and automatically handles the back-and-forth of system events (like `Mask`, `Lock`, `ShareSecrets`) that are generated by effects like randomization.

### **Integration Test Example Flow**

The integration test for the `draw-card` example (`examples/draw-card/src/integration_test.rs`) is the best reference. Here's a conceptual overview:

1.  **Setup**: Create `TestClient` instances for each participant. Use `TestContextBuilder` to configure and build the initial `GameContext` and `TestHandler`.

    ```rust
    use race_test::prelude::*;
    use crate::{MyGame, MyGameData}; // Your game's specific types

    #[test]
    fn test_full_game() -> anyhow::Result<()> {
        // 1. Setup clients
        let mut alice = TestClient::player("Alice");
        let mut bob = TestClient::player("Bob");
        let mut transactor = TestClient::transactor("Transactor");

        // 2. Setup the initial game context using the builder
        let (mut ctx, _) = TestContextBuilder::default()
            .with_data(MyGameData { blind_bet: 100 })
            .with_max_players(2)
            .set_transactor(&mut transactor)
            .add_player(&mut alice, 10000) // Alice joins with 10000 chips
            .build_with_init_state::<MyGame>()?;
    ```
2.  **Simulate Events**: Create and handle events as they would occur in a real game.

    ```rust
    // 3. Bob joins the game. The handler should dispatch a GameStart event.
    let join_event = ctx.join(&mut bob, 10000);
    let event_effects = ctx.handle_event(&join_event)?;
    assert!(event_effects.start_game);

    // 4. Process the dispatched GameStart event.
    ctx.handle_dispatch_event()?;
    ```
3.  **Use the Simulation Loop**: For complex interactions like randomization, instead of handling each system event manually, use `handle_until_no_events`. This powerful function simulates the entire back-and-forth between the game handler and the clients until the game is waiting for the next player action.

    ```rust
    // 5. The game has started, and randomization is pending. Let the test harness handle it.
    // This will simulate the Mask, Lock, and ShareSecrets flow automatically.
    ctx.handle_until_no_events(vec![&mut transactor])?;

    // The game should now be waiting for Alice to bet.
    let state = ctx.state();
    assert_eq!(state.stage, GameStage::Betting);

    // 6. Alice makes her move.
    let bet_event = alice.custom_event(GameEvent::Bet(100));

    // 7. Run the simulation loop again to process the bet and any resulting events.
    ctx.handle_event_until_no_events(&bet_event, vec![&mut alice, &mut bob, &mut transactor])?;

    // Now the state should be waiting for Bob to react.
    let state = ctx.state();
    assert_eq!(state.stage, GameStage::Reacting);
    assert_eq!(state.pot, 200); // 100 blind + 100 bet

    Ok(())
    ```

By following this pattern, you can write concise and powerful integration tests that cover the entire lifecycle of your game, ensuring all logic, state transitions, and settlements work as expected.
