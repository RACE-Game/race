## Project Structure

This project is implemented as a few parts.

### The Transactor

[Code of Transactor](https://github.com/RACE-Game/race/tree/master/transactor)

Transactor is the server acts as the backend. At least one server is required for running games, but it's possible to have multiple servers serving one game at the same time. In this case, they act as different nodes and do randomization together.

### The API & Test Crates

[Code of API crate](https://github.com/RACE-Game/race/tree/master/api)
[Code of Test crate](https://github.com/RACE-Game/race/tree/master/test)

We build games on RACE and package them as WebAssembly bundles, publish them as NFTs. The crate `api` is the one we need when building the game logic, it provides interfaces for the game handler, and the interfaces for settlements, randomization and many more. The crate `test` is the one we use when writing unit tests and integration tests.

### The Facade Server

[Code of Facade](https://github.com/RACE-Game/race/tree/master/facade)

A mock server to take the role of blockchain in testing. In production, instead of using a database, we save everything on-chain. This makes the local development to be tricky and complex. The facade server is useful when we want to test our games locally without touching a blockchain.

### The Command Line Interface

[Code of CLI](https://github.com/RACE-Game/race/tree/master/cli)

The command line tools to manage on-chain data. We use it to manage registrations(game lobbies), games and publish game bundles.

### The TypeScript SDK

[Code of SDK](https://github.com/RACE-Game/race/tree/master/js)

The SDK for building game frontend. It contains everything for interacting with contracts and transators.

### The Race Solana Contract

[Code of Solana Contract](https://github.com/RACE-Game/race-solana)

The contract supports instructions for data management, game settlement and payment.

### Some game implementations

- A simple draw card game: [Link](https://github.com/RACE-Game/race/tree/master/examples/draw-card)
- Texas Hold'em: [Link](https://github.com/RACE-Game/race-holdem)
- Durak: [Link](https://github.com/RACE-Game/durak)
