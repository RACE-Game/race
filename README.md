---
description: Welcome to RACE DAO official gitbook.
---

# ❤️ Official Docs & Links

Race Protocol is a **multi-chain infrastructure designed to facilitate the development and deployment of secure and fair web3 games, particularly asymmetric competitive games**. It aims to address the challenges faced by web3 builders and players by providing a comprehensive set of tools and features that ensure transparency, fairness, and extensibility. The protocol's name, RACE, stands for "Redefining the Best."

**For web3 builders**, Race Protocol offers:

* **Simplified Development**: By providing a clear programming model centered around a "Game Handler," developers can focus on core game logic without becoming experts in blockchain intricacies. The protocol abstracts away complex on-chain interactions and server management.
* **A Serverless Solution**: Community-hosted Transactor and Validator nodes offer developers a ready-to-use backend, removing the need to manage their own server infrastructure. This significantly lowers the barrier to entry for creating and deploying decentralized games.
* **Multi-chain Support**: Race Protocol is designed to be chain-agnostic. The `transport` layer is built with modules for various blockchains including Solana, Sui, and a generic EVM framework, allowing developers to deploy their games across a wide range of ecosystems
* **Transparency and Security**: All critical game data and transactions are recorded on-chain, ensuring transparency and immutability.

**For web3 players**, Race Protocol ensures:

* **Fairness**: The protocol's design guarantees fair play by utilizing peer-to-peer (P2P) randomization and encrypted communication to protect hidden information in asymmetric games. This prevents any single party, including the server operators, from manipulating game outcomes.
* **Security**: Players' assets are protected by smart contracts. Funds are never held by a centralized service; every deposit and commission is 100% transparent and managed on-chain.
* **Accessibility**: The protocol's architecture decouples the game's core logic from its frontend presentation. This allows for a single game backend to be accessed from various platforms, including traditional dApps or metaverse applications, increasing accessibility and player engagement.

By addressing the needs of both builders and players, Race Protocol aims to unlock the potential of web3 gaming and foster a thriving ecosystem of innovative and trustworthy games.

### Key Features

Race Protocol stands out for its key features that cater to the demands of web3 gaming:

* **Multi-chain Infrastructure**: The protocol is engineered for interoperability across different blockchains. The modular `transport` crate is designed to support multiple chains, with concrete implementations already available for Solana, Sui, and a local Facade server for testing. This enables developers to choose the best environment for their game and audience.
* **Decoupled Architecture (Game Core vs. Frontend)**: Race Protocol separates the core game logic from the user interface. The game logic is compiled into a WebAssembly (WASM) module, known as a "Game Bundle," and published on-chain as an NFT. This allows any number of frontends to be developed for a single game, promoting reusability and creative freedom for UI/UX developers.
* **Secure and Fair Gameplay**: At its core, the protocol is built for competitive games where information is not shared equally among all players.
  * **P2P Randomization:** A mental poker-style algorithm implemented in `race-core` ensures that random events (like shuffling cards) are generated through a collaborative and verifiable process between multiple servers (Transactors and Validators), making them tamper-proof.
  * **Encrypted Communication:** The `race-encryptor` module uses a combination of ChaCha20 and RSA to protect sensitive game data and hidden knowledge, ensuring that private information is only revealed to the intended players at the appropriate time.
* **Transparent and Trustless Asset Management**: Player assets are held in on-chain accounts controlled exclusively by the smart contract. The protocol also introduces **Recipient Accounts** (`race-core/src/types/accounts/recipient_account.rs`), which allow for complex, programmable payment flows for handling prize pools, commissions, and affiliate payouts in a transparent and automated manner.
* **Simplified Development with a Game Handler Model**: Developers write their game logic in Rust by implementing the `GameHandler` trait found in the `race-api` crate. This model abstracts away the complexities of blockchain interaction, allowing developers to focus on the game's rules and state transitions. The `Effect` struct provides a clean, side-effect-free way to request actions like generating randomness or triggering settlements from the protocol's runtime.

These features combine to create a robust and reliable infrastructure for building and playing web3 games with enhanced security, fairness, and flexibility.

### Project Status

Race Protocol is under active development and has reached a stage where its core components and SDKs are available for use. The project is open-source and welcomes community involvement.

**Published Packages:**

The protocol is divided into several packages available on standard registries:

**TypeScript (NPM):**

| Package                     | Version                                                                                                                | Description                                   |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| @race-foundation/borsh      | [https://www.npmjs.com/package/@race-foundation/borsh](https://www.npmjs.com/package/@race-foundation/borsh)           | A borsh implementation with decorator support |
| @race-foundation/sdk-core   | [https://www.npmjs.com/package/@race-foundation/sdk-core](https://www.npmjs.com/package/@race-foundation/sdk-core)     | Core SDK for the RACE Protocol                |
| @race-foundation/sdk-solana | [https://www.npmjs.com/package/@race-foundation/sdk-solana](https://www.npmjs.com/package/@race-foundation/sdk-solana) | SDK integration for the Solana blockchain     |
| @race-foundation/sdk-facade | [https://www.npmjs.com/package/@race-foundation/sdk-facade](https://www.npmjs.com/package/@race-foundation/sdk-facade) | SDK integration for the local facade server   |

**Rust (Crates.io):**

| Package         | Version                                                                              | Description                               |
| --------------- | ------------------------------------------------------------------------------------ | ----------------------------------------- |
| race-api        | [https://crates.io/crates/race-api](https://crates.io/crates/race-api)               | API for building a game bundle            |
| race-core       | [https://crates.io/crates/race-core](https://crates.io/crates/race-core)             | Core library with protocol definitions    |
| race-proc-macro | [https://crates.io/crates/race-proc-macro](https://crates.io/crates/race-proc-macro) | Procedural macros for the SDK             |
| race-encryptor  | [https://crates.io/crates/race-encryptor](https://crates.io/crates/race-encryptor)   | Module for encryption and signing         |
| race-client     | [https://crates.io/crates/race-client](https://crates.io/crates/race-client)         | Core logic for player and validator nodes |
| race-test       | [https://crates.io/crates/race-test](https://crates.io/crates/race-test)             | Testkit for game development              |

**Binary Releases:**

Pre-compiled binaries for the following tools are available on the [GitHub releases page](https://www.google.com/url?sa=E\&q=https%3A%2F%2Fgithub.com%2FRACE-Game%2Frace%2Freleases):

* race-transactor: The primary server node for running games.
* race-facade: A local test server that mocks a blockchain for development.
* race-cli: Command-line tools for managing on-chain accounts and assets.

**Smart Contracts:**

| Blockchain | Address                                                                                                                 |
| ---------- | ----------------------------------------------------------------------------------------------------------------------- |
| Solana     | [C3u1cTJGKP5XzPCvLgQydGWE7aR3x3o5KL8YooFfY4RN](https://solscan.io/account/C3u1cTJGKP5XzPCvLgQydGWE7aR3x3o5KL8YooFfY4RN) |

#### Stay tuned and let the games begin!

| [Website](https://race.games/) [(Race.Games)](https://race.games/)            | [<mark style="color:blue;">Discord</mark>](https://discord.gg/raceprotocol) |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| [Github Repo (Main)](https://github.com/RACE-Game/race)                       | [<mark style="color:blue;">Telegram</mark>](https://t.me/racegame)          |
| [<mark style="color:blue;">Medium</mark>](https://medium.com/@race.game.team) | [Twitter](https://twitter.com/RaceGameTeam)                                 |
| [Magic Eden (RACE Heroes)](https://magiceden.io/marketplace/race_heroes)      | [Tensor (RACE Heroes)](https://www.tensor.trade/trade/raceheroes)           |
