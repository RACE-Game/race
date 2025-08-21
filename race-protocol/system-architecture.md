# 🏗️ System Architecture

RACE Protocol is a decentralized infrastructure designed for competitive games. Its architecture separates the core game logic, the server-side coordination, and the on-chain data management into distinct, interacting components. This design ensures that games are transparent, fair, and highly extensible.

***

## The Big Picture

At a high level, the RACE ecosystem consists of four main parts:

1. **On-chain Accounts:** Smart contracts on a supported blockchain (like Solana or Sui) that act as the ultimate source of truth for game state, player assets, and ownership.
2. **Transactor & Validator Nodes:** A cluster of off-chain servers that coordinate real-time gameplay, manage event sequencing, and execute the game logic.
3. **Game Logic (WASM Game Bundles):** The developer-defined game rules, compiled into a portable WebAssembly (WASM) module. This logic is deterministic and runs identically on all nodes.
4. **Client Nodes (Players):** The user-facing applications (e.g., web apps) that allow players to connect to a game, submit actions, and view the game state.

The following diagram illustrates how these components interact:

codeCode

```
+----------------+      Events & Actions      +----------------------+      State Updates      +----------------+
|                | <------------------------> |                      | ---------------------> |                |
|  Client Nodes  |                            |  Transactor/Validator|                        |  Client Nodes  |
|   (Players)    |                            |         Nodes        |                        |   (Players)    |
|                |                            |                      |                        |                |
+----------------+      +--------------------+      +----------------------+      +----------------+
       ^              |                            ^
       |              |                            |
Executes WASM         | Executes WASM              | Submits Transactions
       |              |                            |
       v              v                            v
+-----------------------------+         +--------------------------+
|      WASM Game Bundle       |         |     On-chain Accounts    |
| (Deterministic Game Logic)  | <-----> | (Blockchain Source of Truth) |
+-----------------------------+         +--------------------------+
```

***

## On-chain Accounts

RACE Protocol uses the blockchain as its primary database, ensuring that all critical game data is transparent, verifiable, and decentralized. This eliminates the need for a traditional centralized database, simplifying the architecture and enhancing trust. The core data structures for these accounts are the canonical source of truth for the protocol and are defined in the race-core/src/types/accounts/ directory.

### **Game Account**&#x20;

* **`GameAccount` Defined in:** race-core/src/types/accounts/game\_account.rs

The Game Account is the most central and dynamic account in the RACE ecosystem. It represents a single game instance or "room" and stores all the essential public information required to participate in and verify a game.

#### Key fields include:

* **Core Properties**: `title`, `bundle_addr` (the on-chain address of the Game Bundle NFT), and `token_addr` specify the game's identity and the primary token used for gameplay.
* **Participant Lists**: players and servers are lists that store the public keys and join information for every participant.
* **State & Synchronization**:
  * `access_version`: A counter that increments each time a player or server joins, used to synchronize game state.
  * `settle_version`: A counter that increments with each successful on-chain settlement, versioning the game's financial state.
  * `deposits`: A list of in-game deposits made by players, tagged with the access\_version at which they occurred.
* **Game Logic & Rules**:
  * `data`: A `Vec<u8>` containing the borsh-serialized, game-specific initial configuration (e.g., poker blinds, round times).
  * `entry_type`: An `enum` defining how players can join a game, such as by Cash deposit, a Ticket, or Gating via an NFT.
  * `entry_lock`: An `enum` that allows the game host to control joining and depositing, for example, to stop new players from entering a tournament late.
* **Financial State**:
  * `recipient_addr`: The address of the RecipientAccount that will receive funds like rake or fees.
  * `bonuses`: A list of potential non-monetary awards (like NFTs) that can be distributed during settlement.
  * `balances`: A snapshot of each player's in-game balance, which is updated on-chain only during a settlement.
* **Security & Verification**:
  * `transactor_addr`: The address of the server currently acting as the Transactor for the game.
  * `checkpoint_on_chain`: A cryptographic hash (root) of the last settled game state, allowing anyone to verify the integrity of the off-chain state provided by the Transactor.
  * `votes`: A record of votes cast by servers or clients against an unresponsive Transactor.

### **Game Bundle**

* **`GameBundle` Defined in:** race-core/src/types/accounts/game\_bundle.rs

A Game Bundle is an on-chain NFT that represents the game's logic. It does not store the WASM code directly on-chain. Instead, it contains a uri field that points to a metadata file on a decentralized storage network like Arweave. This metadata file, in turn, points to the actual compiled WASM bundle, ensuring the game logic is immutable and publicly accessible.

### **Player Profile**

* **`PlayerProfile` Defined in:** race-core/src/types/accounts/player\_profile.rs

This on-chain account serves as a player's identity across the RACE ecosystem. A player must create a profile before joining any game. It is a simple structure containing the player's nick (nickname) and an optional pfp (profile picture), which can be the address of an NFT.

### **Registration Account**

* **`RegistrationAccount` Defined in:** race-core/src/types/accounts/registration\_account.rs

A Registration Account acts as a public lobby or directory for games. Game hosts can register their GameAccounts here, and Transactor/Validator nodes monitor these registries to discover new games to serve. A registry can be public (allowing any game to be listed) or private (controlled by a specific owner).

### **Recipient Account**

* **`RecipientAccount` Defined in:** race-core/src/types/accounts/recipient\_account.rs

This is a powerful on-chain treasury designed to handle complex payment flows automatically. It contains one or more slots, where each RecipientSlot can hold a specific token type (or NFTs) and has a list of shares that define how the assets in that slot should be distributed among different owners. This is the core mechanism for transparently handling prize pools, commissions, and affiliate payouts.

### **Server Account**

* **`ServerAccount` Defined in:** race-core/src/types/accounts/server\_account.rs

A simple on-chain account that registers a server node. It contains the server owner's wallet addr and its public network endpoint, allowing clients and other nodes to connect to it.

## Transactor & Validator Nodes

Games on the RACE Protocol are orchestrated by a cluster of off-chain server nodes that ensure real-time gameplay and maintain the integrity of the game state. These nodes, defined in the transactor/ crate, can operate in one of two modes: **Transactor** or **Validator**.

The first server to successfully join and register itself on a GameAccount becomes the authoritative **Transactor**. All subsequent servers that join the same game operate as **Validators**.

***

### **Transactor Mode**

The Transactor is the central coordinator for a live game instance. It is the only node that communicates directly with players and submits transactions to the blockchain. Its primary responsibilities are managed by distinct, modular components within its software:

* **Event Reception & Broadcasting (`component/broadcaster.rs`)**: It receives actions (as `Custom` events) from players' clients and broadcasts the verified sequence of all game events to every connected client and Validator. This ensures all participants share a synchronized view of the game's progress.
* **State Calculation & Logic Execution (`component/event_loop.rs`, `component/wrapped_handler.rs`)**: It maintains the official off-chain game state, known as the GameContext. For each incoming event, it executes the game's WASM logic (`GameHandler`) to calculate the new state and determine any resulting effects.
* **Synchronization with Blockchain (`component/synchronizer.rs`)**: It continuously monitors the on-chain `GameAccount` for new players, new servers, or in-game deposits, and integrates these updates into the `GameContext`.
* **Randomization Coordination**: It initiates and orchestrates the multi-party computation protocol for generating fair and verifiable randomness, collaborating with all connected Validators.
* **Transaction Submission (`component/submitter.rs`)**: It is the only node authorized to sign and submit settlement transactions to the blockchain. When a checkpoint is triggered by the `GameHandler`, the Submitter constructs the transaction to update on-chain balances, eject players, and record the new state checkpoint.

### **Validator Mode**

Validators are crucial for decentralization and security. They act as independent auditors, ensuring the Transactor operates honestly and remains available.

* **State Verification**: Validators connect directly to the Transactor and receive the exact same stream of `BroadcastFrame` events as player clients. They independently run the same WASM game logic to compute their own version of the `GameContext`. If their calculated state ever diverges from the state hash broadcast by the Transactor, they can flag a dispute.
* **Randomization Participation**: Validators are essential participants in the P2P randomization protocol. By contributing their own secret inputs to the process, they ensure that the Transactor cannot predict or manipulate random outcomes (like a card shuffle), guaranteeing cryptographic fairness.
* **Voting for Inactivity (`component/voter.rs`)**: If a Validator detects that the Transactor has gone offline (e.g., by not receiving heartbeats or new event broadcasts), it can call the `vote` instruction on the smart contract. If a sufficient number of Validators vote, the game is programmatically halted, protecting player funds and allowing for a new Transactor to be designated.

***

## Game Logic (WASM Game Bundles)

One of the innovations of RACE is the separation of game logic from the infrastructure. Developers write their game's rules as a self-contained, deterministic state machine in Rust.

* **Implementation:** The game logic is a Rust library that implements the `GameHandler` trait defined in `race-api/src/engine.rs`. This trait requires three key functions: `init_state`, `handle_event`, and `balances`.
* **Compilation:** This library is compiled into a WebAssembly (WASM) binary. This makes the game logic portable and executable in a sandboxed environment on servers and in browsers.
* **Distribution:** The final WASM binary is uploaded to a decentralized storage solution like Arweave. An on-chain `GameBundle` account (see `race-core/src/types/accounts/game_bundle.rs`) is then created, which acts as an NFT, pointing to the WASM bundle's URI. This decouples the game logic from any single platform and allows it to be reused across different frontends or even other games.

Both Transactor/Validator nodes and client nodes load and execute this same WASM bundle, guaranteeing that state transitions are calculated identically everywhere.

***

## Client Nodes (Players)

Clients are the user-facing applications that players use to interact with a game. This could be a web application, a desktop client, or a metaverse integration. The `race-client` crate contains the core logic for a client node.

The primary responsibilities of a client are:

* **Connection Management:** Establish and maintain a WebSocket connection to a game's Transactor.
* **Submitting Actions:** When a player makes a move, the client application packages this action into a `Custom` event and sends it to the Transactor via a `SubmitEvent` call.
* **State Synchronization:** The client receives a stream of `BroadcastFrames` from the Transactor. It uses these frames to update its local copy of the game state, which is then used to render the UI.
* **Cryptography:** Each client manages its own `SecretState` (`race-core/src/secret.rs)`. This is used for cryptographic operations such as creating hidden, immutable decisions (e.g., choosing a move in Rock-Paper-Scissors) and decrypting information revealed by the server (e.g., seeing your cards in poker).

