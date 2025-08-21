---
description: >-
  This section explains the fundamental building blocks and design principles of
  the RACE JS SDK. Understanding these concepts will help you use the SDK
  effectively.
---

# 🧠 Core Concepts

## The Transport Layer (ITransport)

The Transport Layer is the communication bridge between your application and the blockchain. It is defined by the ITransport interface, which abstracts away the specific details of interacting with different blockchain networks like Solana or Sui.

### **Key Purpose:**

* **Blockchain Agnostic:** It allows the core SDK logic (sdk-core) to remain independent of any single blockchain. You can write your game's client-side logic once and switch between different blockchains by simply changing the transport instance.
* **Defines Communication:** It specifies a standard set of methods for all essential on-chain interactions, such as:
  * Fetching account data (getGameAccount, getPlayerProfile).
  * Sending transactions (createGame, join, deposit).
  * Interacting with wallet adapters.

### **Implementations:**

The SDK provides several implementations of the ITransport interface:

* SolanaTransport: For interacting with the Solana blockchain.
* SuiTransport: For interacting with the Sui blockchain.
* FacadeTransport: A mock transport for local development and testing. It simulates blockchain interactions without needing a real network connection or wallet, making it perfect for UI development and quick prototyping.

By depending on the ITransport interface rather than a concrete implementation, your application becomes more modular and easier to test.

***

## Account Models

Account Models are TypeScript classes that represent the on-chain data structures of the RACE Protocol. When you fetch data using the Transport Layer, it is deserialized into these structured objects, giving you type-safe access to game and player information.

The most important models include:

* **`GameAccount`**: This is the central object representing a single game instance. It holds the complete public state of the game, including:
  * **Metadata**: `addr`, `title`, `bundleAddr` (the game's code), and `tokenAddr` (the currency used).
  * **Participants**: Lists of `players` and `servers` currently involved.
  * **State**: The current `accessVersion` and `settleVersion`, which track player entry and financial state changes respectively.
  * **Transactor**: The `transactorAddr` of the server currently responsible for processing events.
  * **Rules**: The `entryType` (e.g., cash buy-in, ticket, or NFT-gated) and `maxPlayers`.
  * **On-Chain Checkpoint**: `checkpointOnChain` stores a cryptographic hash of the game's state, ensuring data integrity and verifiability.
* **`PlayerProfile`**: Represents a player's on-chain identity. It contains their addr (wallet address), `nick` (nickname), and an optional `pfp` (profile picture NFT address).
* **`GameBundle`**: Contains the metadata for a game's playable code, including a uri pointing to the WebAssembly (Wasm) binary that holds the game logic.
* **`Token` & `Nft`**: Standardized objects for fungible tokens and non-fungible tokens (NFTs), used for entry fees, gating access, and distributing rewards.
* **`RecipientAccount`**: Acts as the game's treasury. It automatically collects and distributes fees or winnings to multiple parties based on predefined rules in its slots. Each slot can define different shares for various owners.

***

## Event-Driven Architecture

The RACE Protocol operates on a deterministic, event-driven architecture. This ensures that game logic is predictable and verifiable. All state changes are the result of processing a sequence of events.

### **The flow is as follows:**

1. **GameEvent**: An action or occurrence is represented as a GameEvent. This could be a player joining (Join), a custom game action (Custom), or an internal timeout (ActionTimeout). All inputs to the game logic are formalized as events.
2. **Handler**: The Handler (which runs the game's Wasm binary) is a pure function. It takes the current GameContext (the game's state) and a GameEvent as input.
3. **Effect**: The Handler processes the event and produces an Effect. An Effect is a data structure that describes all the required state changes, such as updating a player's balance, initiating a timeout, or creating a cryptographic checkpoint.
4. **GameContext Update**: The AppClient receives this Effect and applies its changes to the local GameContext, ensuring the client's state is synchronized with the game's official state.

This Event -> Handler -> Effect loop is the core of all state transitions in the RACE SDK. It guarantees that as long as everyone processes the same events in the same order, they will all arrive at the exact same game state.

***

## Handling Asynchronous Actions (`ResponseStream`)

Blockchain transactions are not instantaneous. They go through several stages: creation, signing by the user, submission to the network, and final confirmation. To provide a clear and responsive user experience, your application needs to track this lifecycle.

The RACE SDK simplifies this with the **`ResponseStream`**. Instead of a simple `Promise`, any method in `AppHelper` that creates a transaction (like `createGame`, `join`, or `deposit`) returns a `ResponseStream`, which is an **async generator**. You can iterate over this stream to receive real-time status updates about the transaction's progress.

### **Transaction Lifecycle Statuses**

The `ResponseStream` will yield objects, each with a status property that can be one of the following:

* **`preparing`**: The SDK is preparing the transaction payload.
* **`waiting-wallet`**: The transaction has been sent to the user's wallet and is awaiting their signature. This is the perfect time to show a "Please confirm in your wallet" message.
* **`confirming`**: The transaction has been signed and submitted to the network. The SDK is now waiting for it to be confirmed by the blockchain.
* **`succeed`**: The transaction has been successfully confirmed. The response object will contain a `data` field with the result (e.g., the new game address or the transaction signature).
* **`user-rejected`**: The user rejected the transaction in their wallet.
* **`transaction-failed`**: The transaction was submitted but failed on-chain. The response will include an error object with details.
* **`failed`**: An error occurred before the transaction was sent (e.g., invalid parameters).

### **Example: Using `ResponseStream`**

The for `await...of` loop is the standard way to consume a `ResponseStream`. This pattern is ideal for updating UI components in frameworks like React or Vue.

```typescript
import { AppHelper } from '@race-foundation/sdk-core';
import { FacadeWallet, FacadeTransport } from '@race-foundation/sdk-facade';

// Assume `helper` and `myWallet` are already initialized.
const helper = new AppHelper(new FacadeTransport());
const myWallet = new FacadeWallet();

async function joinGame(gameAddr: string, depositAmount: bigint) {
  // Mock keys for the facade. In a real app, these come from an Encryptor.
  const mockKeys = { ec: 'mock-key', rsa: 'mock-key' };

  const joinStream = helper.join(myWallet, {
    addr: gameAddr,
    amount: depositAmount,
    keys: mockKeys,
  });

  console.log('Attempting to join game...');

  for await (const response of joinStream) {
    if (!response) break;

    // You can update your UI based on the status
    switch (response.status) {
      case 'preparing':
        console.log('Preparing transaction...');
        break;
      case 'waiting-wallet':
        console.log('Please approve the transaction in your wallet.');
        break;
      case 'confirming':
        console.log(`Transaction submitted! Signature: ${response.signature}. Waiting for confirmation...`);
        break;
      case 'succeed':
        console.log('Successfully joined the game!', response.data);
        return; // The process is complete.
      case 'failed':
      case 'user-rejected':
      case 'transaction-failed':
        console.error(`Action failed with status "${response.status}":`, response.error);
        return; // The process has ended with an error.
    }
  }
}
```

## Cryptography and The `Encryptor`

Security and fairness are built into the core of the SDK through a robust cryptographic layer managed primarily by the `Encryptor` class. This class abstracts away all complex cryptographic operations so you can focus on your game logic.

### **Core Responsibilities of the `Encryptor`**

* **Key Management**: The `Encryptor` manages two essential key pairs for each player:
  * **ECDSA Keys**: Used for signing transactions and events to prove identity and authorship.
  * **RSA Keys**: Used for encrypting and decrypting secrets, which is crucial for the protocol's fair random number generation.
* **Signing & Encryption**: It provides simple methods to sign messages and encrypt/decrypt game state secrets, handling the low-level cryptographic primitives for you.
* **Key Persistence**: The Encryptor can be initialized with an optional Storage provider to securely cache a player's generated keys in the browser's IndexedDB. This is a crucial feature for a good user experience, as it prevents players from having to regenerate and re-authorize keys every time they visit your application.

### **Creating and Using the `Encryptor`**

You should create an `Encryptor` instance for the current player when their session begins.

```typescript
import { Encryptor, Storage } from '@race-foundation/sdk-core';

// 1. Initialize a storage provider (uses IndexedDB by default)
const storage = new Storage();

// 2. Create the encryptor.
// This will either load existing keys from storage or generate new ones
// and cache them for future sessions.
const playerAddress = 'PLAYER_WALLET_ADDRESS';
const encryptor = await Encryptor.create(playerAddress, storage);

console.log('Encryptor is ready.');

// 3. Export public keys to use in transactions.
// For example, the `join` method requires the player's public keys.
const publicKeys = await encryptor.exportPublicKey();

// Now, `publicKeys` can be passed to methods like `appHelper.join()`.
// appHelper.join(wallet, { ..., keys: publicKeys });
```

### **Fair Randomness (Multi-Party Computation)**

To prevent any single party (even the server) from predicting or controlling random outcomes, the SDK uses a commit-reveal scheme. While the process is complex, the `Encryptor` handles it automatically. The high-level flow is:

1. All participating servers contribute to a random outcome by providing encrypted secrets (`Lock` and `Mask` events).
2. Once all secrets are committed, they are shared (`ShareSecrets` event) and combined to produce a verifiably random number.
3. The `AppClient` and `Encryptor` handle the complex decryption process behind the scenes, making the revealed results available in the `GameContextSnapshot`.

## Caching with the `Storage` Interface

To enhance performance and reduce redundant network requests, the SDK includes a pluggable caching layer defined by the `IStorage` interface. This allows you to cache frequently accessed, semi-static on-chain data directly in the user's browser.

The SDK provides a default `Storage` class that implements this interface using **IndexedDB**, a standard browser database.

### **What Does It Cache?**

The `Storage` class can cache:

* **Token Metadata**: Information like a token's name, symbol, and icon URL.
* **NFT Metadata**: Details for NFTs, including their image, name, and collection.
* **Player Profiles**: Nicknames and PFP addresses.
* **Encryptor Keys**: The player's generated cryptographic keys for persistence across sessions.

### **How to Use the `Storage` Interface**

Using the cache is simple. You instantiate the `Storage` class and pass it as an optional argument to the relevant `AppHelper` or `Encryptor` methods.

```typescript
import { AppHelper, Storage } from '@race-foundation/sdk-core';
import { SolanaTransport } from '@race-foundation/sdk-solana';

// 1. Create a single instance of the Storage provider for your app.
const storage = new Storage();

const transport = new SolanaTransport(/*...*/);
const appHelper = new AppHelper(transport);
const playerAddress = 'PLAYER_WALLET_ADDRESS';

async function fetchProfileWithCaching() {
  // 2. Pass the `storage` instance to the method.
  // The first time this is called, it will fetch from the network and cache the result.
  // Subsequent calls will return the cached data instantly.
  const profile = await appHelper.getProfile(playerAddress, storage);

  if (profile) {
    console.log(`Profile for ${profile.nick} loaded from cache or network.`);
  }
}

fetchProfileWithCaching();
```

By leveraging the `Storage` interface, you can make your application feel significantly faster and more responsive, especially when dealing with assets and profiles that don't change often.
