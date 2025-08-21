---
description: An alphabetical list of key terms and concepts within the RACE Protocol.
---

# 📚 Glossary

### **Access Version**

A sequential `u64` counter on the `GameAccount` that increments each time a new player or server joins a game. It is used to ensure all nodes in the game network can deterministically synchronize to the same state, especially when loading a game from a checkpoint. Each player and server is tagged with the `access_version` at which they joined.

### **API (`race-api` crate)**

The primary Rust crate that defines the public-facing interface for developing game logic bundles (WASM). It contains the essential `GameHandler` trait, the `Effect` and `Event` structs, and other core types necessary for game development.

### **Award**

An `Effect` that grants a player a non-monetary bonus, identified by a string. This is used for issuing prizes like NFTs or special items, which are then processed during settlement. Defined in `race-api/src/types.rs`.

### **Blacklist**

A feature of the Transactor that maintains a list of malformed or problematic game addresses. Games on this list will not be loaded or served. This is a safety mechanism to prevent the Transactor from interacting with faulty game accounts. Implemented in `transactor/src/blacklist.rs`.

### **Borsh**

A binary serialization format used extensively throughout the RACE Protocol for its compact size and canonical representation, ensuring deterministic serialization of data structures.

### **Bridge Event**

A mechanism for communication between a main game and its sub-games. A `GameHandler` can emit a bridge event to a specific sub-game via an `Effect`, allowing for complex, multi-stage game logic. Defined by the `BridgeEvent` trait in `race-api/src/event.rs`.

### **Checkpoint**

A complete snapshot of a game's state at a specific moment. When a `GameHandler` triggers a checkpoint via an `Effect`, the current state is serialized and prepared for on-chain settlement. Checkpoints enable game state recovery and are crucial for the settlement process. The core checkpoint logic is in `race-core/src/checkpoint.rs`.

### **CLI (`race-cli` crate)**

A command-line interface for managing and interacting with on-chain RACE Protocol accounts. It allows developers and hosts to perform actions like publishing game bundles, creating game accounts, registering games to a lobby, and querying on-chain data.

### **Custom Event**

A game-specific event defined by the game developer. These events are wrapped in the `Event::Custom` variant and are the primary way for players to send actions (e.g., "Bet", "Fold") to the GameHandler. Custom events must implement the CustomEvent trait from `race-api/src/event.rs`.

### **Decision**

A feature that allows a player to commit to an immutable, hidden action (e.g., choosing Rock, Paper, or Scissors). The decision is initially encrypted and can only be revealed later when the player shares the secret key. This is managed through the `DecisionStatus` enum and ask/release calls in the `Effect` struct. See `race-api/src/decision.rs`.

### **Effect (`Effect` struct)**

The primary bridge for communication from the `GameHandler` (WASM) to the Transactor runtime. It allows the pure-function `GameHandler` to trigger side effects like generating randomness, managing timeouts, making settlements, interacting with sub-games, and logging, without directly accessing system resources. Defined in `race-api/src/effect.rs`.

### **Entry Type**

An enum defining the requirements for a player to join a game. Options include `Cash` (depositing a range of tokens), `Ticket` (paying a fixed amount), or `Gating` (holding a specific NFT). Defined in `race-core/src/types/accounts/game_account.rs`.

### **Event (`Event` enum)**

A data structure representing a discrete action or state change within the game. The `GameHandler`'s logic is driven by processing a sequence of these events. Events can be system-level (like `GameStart`, `Join`, `SecretsReady`) or game-specific (`Custom`). Defined in `race-api/src/event.rs`.

### **Facade (`race-facade` crate)**

A mock server that emulates a blockchain environment for local development and testing. It allows developers to test their game bundles and client applications without needing to connect to a live or local blockchain network.

### **Game Account (`GameAccount` struct)**

The central on-chain account that represents a single game instance or room. It stores all public information about the game, including its title, the associated GameBundle address, the list of players and servers, current `access_version` and `settle_version`, and the latest on-chain checkpoint. Defined in `race-core/src/types/accounts/game_account.rs`.

### **Game Bundle (`GameBundle` struct)**

An on-chain account (typically an NFT) that contains the URI pointing to the game's compiled WASM logic, which is stored on a decentralized storage solution like Arweave. This decouples the game logic from the game instance itself. Defined in `race-core/src/types/accounts/game_bundle.rs`.

### **Game Handler (`GameHandler` trait)**

A Rust trait that defines the core interface for game logic. Developers implement this trait to create their game bundle. It has two main methods: `init_state` for initialization and `handle_event` for processing game events. The game's state is encapsulated within the struct that implements this trait. Defined in `race-api/src/engine.rs`.

### **Player Profile (`PlayerProfile` struct)**

An on-chain account that stores a player's public information, such as their nickname (`nick`) and an optional avatar NFT (`pfp`). A player must have a profile to join games. Defined in `race-core/src/types/accounts/player_profile.rs`.

### **Randomization (`RandomSpec` enum)**

The process for generating unpredictable, fair randomness, critical for games with hidden information. The `GameHandler` requests randomness via `effect.init_random_state()` with a `RandomSpec` (e.g., `ShuffledList` for a deck of cards). The Transactor and Validators then perform a multi-party computation (a variant of Mental Poker) to generate it. The result is revealed to the `GameHandler` via the `SecretsReady` event. See `race-api/src/random.rs`.

### **Recipient Account (`RecipientAccount` struct)**

An on-chain account designed to simplify complex payment distributions. It holds funds and distributes them into different `slots` based on predefined `shares`. This is useful for handling tournament prize pools, commissions, or other multi-party payment scenarios. Defined in `race-core/src/types/accounts/recipient_account.rs`.

### **Registration Account (`RegistrationAccount` struct)**

An on-chain account that acts as a game lobby or directory. Game hosts can register their `GameAccount`s here, and Transactors can discover new games to serve by monitoring these registries. Defined in `race-core/src/types/accounts/registration_account.rs`.

### **Settle**

An `Effect` that defines a change in a player's token balance (add or subtract) or their status in the game (eject). A collection of `Settle` effects is processed by the Transactor during a settlement transaction. Defined in `race-api/src/types.rs`.

### **Settle Version**

A sequential `u64` counter on the `GameAccount` that increments after each successful settlement transaction. It acts as a version number for the game's financial state and is used to handle player deposits that occur mid-game correctly.

### **Sub-game**

A game instance launched by a parent (main) game. This allows for complex game structures, like a poker tournament where a main lobby (`GameHandler`) launches individual table games (`GameHandler` sub-games). Communication occurs via Bridge Events.

### **Transactor (`race-transactor` crate)**

The primary server node in the RACE network. It is responsible for orchestrating gameplay: receiving events from players, feeding them to the `GameHandler`, broadcasting state changes, coordinating randomness with Validators, and submitting settlement transactions to the blockchain.

### **Validator**

A server node that participates in the network alongside the Transactor. Its main roles are to validate the Transactor's actions by independently processing the same event stream and to participate in the multi-party computation for randomization, ensuring fairness and decentralization.

### **WASM (WebAssembly)**

The compilation target for game logic written in Rust. Game bundles are distributed as WASM files, allowing them to be executed securely and deterministically by the Transactor, Validators, and even clients.
