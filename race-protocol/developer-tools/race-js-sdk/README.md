# 📜 RACE JS SDK

## **About the RACE JS SDK**

The RACE Protocol JS SDK is a powerful, modular toolkit designed to help you build games and applications on top of the RACE Protocol. Written in TypeScript, it abstracts away the complexities of blockchain interaction, allowing you to focus on creating engaging user experiences. The SDK provides all the necessary tools to manage game state, handle complex game logic, and interact securely with the underlying blockchain.

It is designed to be chain-agnostic, offering production-ready support for both **Solana** and **Sui**. Whether you are building a simple turn-based game or a complex real-time strategy game, this SDK is your comprehensive solution for bringing decentralized gaming experiences to life on the web.

### **Key Features**

* **✨ Dual-Layer API**: The SDK offers two distinct ways to interact with the protocol, catering to different needs:
  * **High-Level AppHelper**: A simple, stateless API perfect for performing common actions like creating a game, listing player profiles, or joining a table. It's the fastest way to get started.
  * **Low-Level AppClient**: A powerful, stateful client that maintains a persistent connection to a game. It's designed for building rich, real-time user experiences where you need to react instantly to game events.
* **🔗 Seamless Multi-Chain Support**: Built with a modular transport layer, the SDK is designed to be blockchain-agnostic. It currently has robust, production-ready support for both **Solana** and **Sui**, allowing you to write your core application logic once and deploy across different ecosystems.
* **🛡️ Simplified & Secure Cryptography**: In-game secrets, like a player's hand in a card game, are handled securely. The SDK provides a straightforward Encryptor class that manages player key pairs and handles the encryption and decryption of private game state, so you don't have to be a cryptography expert.
* **🧱 Easy Serialization with Borsh**: We provide a custom implementation of the Borsh serialization format. Using simple TypeScript decorators (@field, @variant), you can define complex data structures that are efficiently and reliably serialized for on-chain use.
* **🎮 Development Facade**: To accelerate development and testing, the SDK includes a special @race-foundation/sdk-facade package. It simulates a blockchain environment right in your browser, allowing you to build and test your game's logic without needing to connect to a real wallet or a live network.
* **🔄 Asynchronous Response Handling**: Transactional methods return a ResponseStream (an async generator) that emits status updates throughout the lifecycle of a transaction—from preparing and waiting-wallet to succeed or failed. This allows you to build responsive UIs that give users clear, real-time feedback.
* **⚡ Pluggable Caching Layer**: The SDK includes a built-in Storage interface that can cache frequently accessed on-chain data, such as token metadata, NFTs, and player profiles. The default implementation uses IndexedDB to dramatically improve performance and reduce the number of RPC calls.

***

## **Architecture Overview**

The RACE JS SDK is designed as a **monorepo** composed of several distinct packages. This modular approach separates the core, blockchain-agnostic logic from the chain-specific implementations, making the SDK flexible and extensible.

Here is a high-level view of how the packages fit together:

```
+-------------------------------------+
|      Your Game/Application UI       |
|    (Uses AppHelper or AppClient)    |
+-------------------------------------+
                    |
                    v
+-------------------------------------------+
|      📦 @race-foundation/sdk-core         |
| (Core Logic, State, Events, Interfaces)   |
+-------------------------------+--------------------+--------------------+
|                               |                    |                    |
v                               v                    v                    v
+---------------------+------------------+---------------------+------------------+
| @race-foundation/   | @race-foundation/| @race-foundation/   | @race-foundation/|
|     sdk-solana      |      sdk-sui     |     sdk-facade      |      borsh       |
| (Solana Transport)  |  (Sui Transport) |  (Dev Transport)    | (Serialization)  |
+---------------------+------------------+---------------------+------------------+
```

### **Package Breakdown**

* **📦 @race-foundation/sdk-core**\
  This is the heart of the SDK. It contains all the essential, blockchain-agnostic logic and is the primary package you will interact with. It includes:
  * The core interfaces, like `ITransport`, which define a standard contract for how the SDK communicates with any blockchain.
  * The high-level `AppHelper` for simple, stateless actions.
  * The low-level `AppClient` for building stateful, real-time game experiences.
  * All core data models for accounts, events, and game state (`GameAccount`, `GameEvent`, etc.).
  * The `Encryptor` class for handling cryptography and secure state management.
  * The `Storage` interface for optional browser-side caching of on-chain data.
* **🔗 @race-foundation/sdk-solana & @race-foundation/sdk-sui**\
  These are the chain-specific "drivers." Each of these packages implements the `ITransport` interface from `sdk-core` for its respective blockchain. They handle all the details of creating, signing, and sending transactions, as well as fetching and parsing on-chain data for either Solana or Sui. You will install the one that corresponds to your target blockchain.
* **🎮 @race-foundation/sdk-facade**\
  This is your local development playground. The facade also implements the `ITransport` interface but does not connect to a real blockchain. Instead, it simulates the behavior of the RACE Protocol locally, using an in-memory state. This is invaluable for rapid prototyping, writing tutorials, and building UI components without needing a wallet or a live network connection.
* **🧱 @race-foundation/borsh**\
  A powerful utility package that handles the serialization and deserialization of data structures into the **Borsh** format. It uses TypeScript decorators (`@field`, `@variant`) to make it easy to define schemas that are compatible with the strict, deterministic format required by the on-chain programs.
* **packages/config** (Internal Package)\
  This is an internal, non-published package that contains shared configuration files for the monorepo, such as `tsconfig.json` for TypeScript, `jest.config.js` for testing, and `.prettierrc` for code formatting. It is primarily used for maintaining code consistency and quality within the SDK's development and is not intended for direct use by consumers of the SDK.
