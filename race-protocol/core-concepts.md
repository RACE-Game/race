---
description: >-
  This section delves into the fundamental principles of the RACE Protocol.
  Understanding these concepts is essential for building robust, secure, and
  fair games on the platform.
---

# 🧠 Core Concepts

## **The Game Handler Model**

The heart of any game built on RACE is the **Game Handler**. It is a self-contained state machine, written in Rust and compiled to WebAssembly (WASM), that encapsulates all of your game's rules, state, and logic. This approach allows developers to focus entirely on creating the game experience, while the RACE runtime handles the complex underlying mechanics.

The `GameHandler` is defined by a simple trait located in `race-repo/api/src/engine.rs`:

```rust
// Source: race-repo/api/src/engine.rs

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Called once to initialize the handler's state. It receives data
    /// from the on-chain game account to configure the game.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Called by the runtime for every event in the game. This is where the
    /// core game logic resides, processing events and updating state.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;

    /// Called by the runtime just before creating a checkpoint. It must
    /// return the current balance of every player in the game.
    fn balances(&self) -> Vec<PlayerBalance>;
}
```

This model is built on several key principles:

* **Purity and Determinism**: A `GameHandler` must be a pure function. This means it has no direct access to external resources like network sockets, clocks, or random number generators. For an identical state and event, it must always produce the exact same new state and effects. This determinism is the cornerstone of the protocol's security, as it allows every node (Transactors, Validators, and Players) to independently run the same logic and arrive at the same conclusion, ensuring the game's integrity.
* **State Management**: The Rust struct that implements the `GameHandler` trait is your game's state. Because the trait requires `BorshSerialize` and `BorshDeserialize`, the runtime can efficiently save and load the handler's entire state between event executions. This makes the game logic stateless from the runtime's perspective.
*   **The `#[game_handler]` Macro**: To simplify development, RACE provides a procedural macro found in the `race-proc-macro` crate. By annotating your state struct with #\[game\_handler], you automatically generate the necessary WebAssembly boilerplate (the Foreign Function Interface, or FFI) that allows the RACE runtime to call into your Rust code.

    codeRust

    ```rust
    // A minimal handler showing the macro in action.
    // Source: race-repo/examples/minimal/src/lib.rs

    use race_api::prelude::*;
    use race_proc_macro::game_handler;

    #[derive(BorshDeserialize, BorshSerialize)]
    #[game_handler] // This macro creates the WASM FFI functions.
    struct Minimal {
        n: u64,
    }
    ```

The `GameHandler`'s only way to interact with the outside worldc—cto request a random number, start a timer, or pay a playerc— is through the `Effect` object, which forms the API to the off-chain runtime.

***

## **Events & Effects (The Off-Chain API)**

The interaction between your game logic and the RACE runtime follows a clear pattern: **Events** are the inputs that drive your state machine, and the **Effect** object is the mechanism you use to request outputs or side effects.

### **The `Event` Enum**

An `Event` represents any discrete action or occurrence that can happen in a game. The `GameHandler`'s primary job is to process a sequence of these events. The full definition can be found in `race-repo/api/src/event.rs`. Key variants include:

* **Player-Driven Events**:
  * `Event::Custom`: The container for all game-specific actions defined by you (e.g., "Bet", "Fold", "CastSpell").
  * `Event::Join`: Sent when new players have successfully joined the on-chain game account.
  * `Event::Leave`: Sent when a player has disconnected or left the game.
  * `Event::Deposit`: Sent when a player adds more funds to the game mid-round.
  * `Event::AnswerDecision`: A player's response to a request for a hidden, binding decision.
* **System-Driven Events**:
  * `Event::GameStart`: Signals the beginning of a new game or round.
  * `Event::ActionTimeout` / `Event::WaitingTimeout`: Triggered when a timer you set via an Effect expires.
  * `Event::RandomnessReady`: Indicates that a requested randomization has been completed by the servers.
  * `Event::SecretsReady`: Indicates that the cryptographic secrets needed to decrypt random values or player decisions are now available.
* **Inter-Game Communication**:
  * `Event::Bridge`: An event sent from a parent game to one of its sub-games, or vice-versa.
  * `Event::SubGameReady`: Signals to a parent game that a requested sub-game has been successfully initialized.

### **The `Effect` Struct: Your API to the Runtime**

The `Effect` object is the most important tool in your development kit. It is passed mutably into `init_state` and `handle_event`, allowing your game logic to request actions from the runtime. Its full definition is in `race-repo/api/src/effect.rs`.

### **State & Checkpoints**

* `effect.checkpoint()`: This is a crucial function. It signals to the runtime that the game has reached a stable, settled state. The runtime will then save a snapshot of the GameHandler state and prepare an on-chain transaction based on any settlement effects you have requested.

### **Settlements & Player Management**

These methods are used to define the financial outcome of a game round and all implicitly trigger a checkpoint.

* `effect.withdraw(player_id, amount)`: Instructs the runtime to pay out amount from a player's in-game balance to their wallet in the final settlement.
* `effect.eject(player_id)`: Removes a player from the game's on-chain account during settlement. Their balance must be handled via withdraw.
* `effect.transfer(amount)`: Transfers amount from the game's pot to the designated on-chain Recipient Account. This is typically used for rake or commissions.
* `effect.award(player_id, bonus_identifier)`: Grants a player a non-fungible bonus (like an NFT prize) identified by a string.
* `effect.accept_deposit(deposit)` / `effect.reject_deposit(deposit)`: Manages player deposits made mid-game, either accepting them into the game's economy or flagging them for an on-chain refund.

### **Randomness & Decisions**

These methods allow you to interact with the protocol's secure multi-party computation features.

* `effect.init_random_state(spec: RandomSpec) -> RandomId`: Requests a new source of randomness. The `RandomSpec` can be a `ShuffledList` (e.g., for a deck of cards) or a Lottery. Returns a `RandomId` that you must store in your state.
* `effect.assign(random_id, player_id, indices)`: Privately assigns a random value to a player. For example, dealing cards to a specific player's hand.
* `effect.reveal(random_id, indices)`: Publicly reveals a random value to all participants, like the flop in poker.
* `effect.get_revealed(random_id)`: After an `Event::SecretsReady`, this allows you to read the plaintext values of revealed random items.
* `effect.ask(player_id) -> DecisionId`: Prompts a player to submit a hidden, unchangeable action.
* `effect.release(decision_id)`: Requests the public reveal of a committed decision.
* `effect.get_answer(decision_id)`: After an `Event::SecretsReady`, this allows you to read the plaintext value of a player's decision.

### **Timeouts & Control Flow**

* `effect.action_timeout(player_id, duration_ms)`: Starts a timer for a specific player. If they don't act in time, an Event::ActionTimeout will be sent to the handler.
* `effect.wait_timeout(duration_ms)`: Starts a general-purpose timer. When it expires, an Event::WaitingTimeout is sent.
* `effect.start_game()`: Requests the runtime to dispatch an Event::GameStart.
* `effect.stop_game()`: Requests the runtime to end the game and dispatch an Event::Shutdown.

### **Sub-Games & Inter-Game Communication**

* `effect.launch_sub_game(bundle_addr, max_players, init_data) -> GameId`: Requests the runtime to launch a new, nested game instance using a specified game bundle.
* `effect.bridge_event(destination_game_id, event)`: Sends a custom `BridgeEvent` to another game instance (parent or sub-game).

### **Logging**

For debugging, you can log messages from your handler to the Transactor's console.

* `effect.info("message")`
* `effect.warn("message")`
* `effect.error("message")`
* `effect.debug("message")`

***

## Synchronization (Access & Settle Versions)

Ensuring that every participant has a consistent view of the game is paramount. Given the asynchronous nature of blockchains, RACE uses two versioning numbers stored in the `GameAccount` to manage synchronization.

* **Access Version**: This is a counter that increments every time a player or server joins the game. Each participant is tagged with the `access_version` at which they joined. When a node initializes or restores its state from a checkpoint, it uses the checkpoint's `access_version` to filter for participants who were present at that time, ignoring anyone who joined later. This ensures all nodes compute the game state based on the same set of participants.
* **Settle Version**: This counter increments with each on-chain settlement. It represents a version of the game's financial state. Player deposits are tagged with the `settle_version` they are intended for. This system prevents double-spending and ensures that state changes and deposits are applied correctly and in the right order, allowing any node to reliably reconstruct the current state from the last on-chain checkpoint and subsequent events.

***

## P2P Randomization

Fairness in competitive games often relies on unpredictable, verifiable randomness. RACE implements a Mental Poker-style algorithm to achieve this without a trusted third party. The process is managed by the Transactor and Validators.

The state of a randomization process is defined by the `RandomStatus` enum in `race-core/src/random.rs`.

1. **Request**: A Game Handler requests randomness via `effect.init_random_state()` with a `RandomSpec` (e.g., a `ShuffledList` for a deck of cards or a Lottery).
2. **Masking**: Each server encrypts the initial set of items with its own unique, private "mask" key. The items are shuffled between each masking step.
3. **Locking**: Each server re-encrypts the now-shuffled and masked items with a set of "lock" keys (one for each item) and publishes the cryptographic digests of these lock keys.
4. **Assignment/Reveal**: The Game Handler can now either assign a specific encrypted item to a player or reveal it publicly. This is a request to the servers.
5. **Secret Sharing**: To decrypt an item, every server shares its corresponding "lock" key for that specific item. For an assigned item, keys are sent privately to the player; for a revealed item, they are broadcast publicly.
6. **Decryption**: Once a player or the handler has all the necessary lock keys for an item, they can decrypt it and discover its value. Because the "mask" keys are never shared, no single server can know the final order of the items.

This multi-stage process ensures that no single server can predict or control the outcome, providing strong guarantees of fairness.

***

## Payment (Recipient Accounts)

To handle complex payment scenarios like tournament prize pools, commissions, and sponsorships, RACE provides **Recipient Accounts**. This system avoids convoluted settlement logic within the Game Handler itself.

* **Structure**: A `RecipientAccount` (`race-core/src/types/accounts/recipient_account.rs`) contains one or more slots.
* **Slots**: Each `RecipientSlot` can be configured for a specific purpose, like holding a particular SPL token or NFT type.
* **Shares**: Within each slot, shares define how the funds or assets are to be distributed. A `RecipientSlotShare` specifies an `owner` (which can be a specific address or an unassigned identifier) and its `weights` for the distribution.

**Use Case: Tournament Payouts**

A tournament Game Handler doesn't need to calculate the prize for each of the top 10 players. Instead, its associated `RecipientAccount` can have a "Prize Pool" slot. The Game Handler simply uses `effect.transfer()` to send the entire prize pool to this account. The shares in that slot would be pre-configured (e.g., 1st place: 50%, 2nd place: 30%, etc.). The winners can then independently call the `recipient_claim` instruction to receive their portion. This makes the Game Handler simpler and the payment logic more modular and transparent.
