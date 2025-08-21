# ✨ High-Level API: The AppHelper

## Introduction

The `AppHelper` is your primary entry point for performing common, stateless actions on the RACE Protocol. It provides a simplified, high-level interface for tasks like creating games, fetching account data, and managing player profiles without needing to manage a persistent real-time connection.

### **Key characteristics of the AppHelper:**

* **Stateless:** Each method call is an independent, one-off operation.
* **Action-Oriented:** Designed for actions like "create," "get," "list," and "join."
* **Blockchain-Agnostic:** The same `AppHelper` methods are used regardless of the underlying blockchain (Solana, Sui, etc.), thanks to the Transport Layer abstraction.
* **Ideal for Web Servers and Scripts:** Perfect for backend services, administrative scripts, or web frontends that only need to perform specific transactions or queries without subscribing to real-time game events.

In contrast, the `AppClient` (covered in [The App Client](low-level-api-the-appclient.md)) is stateful and designed to maintain a persistent WebSocket connection to a specific game instance, making it ideal for building real-time game clients. For many applications, you will use both: the `AppHelper` to discover and create games, and the `AppClient` to play them.

***

## Initialization

To use the `AppHelper`, you must first initialize it with a **Transport**. The transport is the bridge that connects the SDK to a specific blockchain network. The SDK provides several transports out of the box.

The `AppHelper` itself is stateless, so you only need to provide the transport during initialization. The user's wallet is passed into the specific methods that require a transaction to be signed.

### **Initializing with FacadeTransport (for local development)**

The `FacadeTransport` is a mock transport that simulates a blockchain environment, making it perfect for local development, tutorials, and testing without needing a real wallet or connection to a live network.

```typescript
import { AppHelper } from '@race-foundation/sdk-core';
import { FacadeTransport } from '@race-foundation/sdk-facade';

// The FacadeTransport does not require any arguments.
const transport = new FacadeTransport();
const appHelper = new AppHelper(transport);

console.log('AppHelper is ready for local development!');
```

### **Initializing with SolanaTransport**

To connect to the Solana blockchain, use the `SolanaTransport`.

```typescript
import { AppHelper } from '@race-foundation/sdk-core';
import { SolanaTransport } from '@race-foundation/sdk-solana';

// The chain identifier for Solana Mainnet.
const SOLANA_CHAIN = 'solana:mainnet'; 

// Your preferred Solana RPC endpoint.
const SOLANA_RPC_ENDPOINT = 'https://api.mainnet-beta.solana.com';

const solanaTransport = new SolanaTransport(SOLANA_CHAIN, SOLANA_RPC_ENDPOINT);
const appHelper = new AppHelper(solanaTransport);

console.log('AppHelper is connected to Solana!');
```

### **Initializing with SuiTransport**

To connect to the Sui blockchain, use the `SuiTransport`.

```typescript
import { AppHelper } from '@race-foundation/sdk-core';
import { SuiTransport } from '@race-foundation/sdk-sui';

// Your preferred Sui RPC endpoint.
const SUI_RPC_ENDPOINT = 'https://fullnode.mainnet.sui.io:443';

// The on-chain address of the deployed RACE package
const RACE_PACKAGE_ID = "0x1d69af8651c81c19eeca3411f276177f3627ffb5a3da6851a3f9257f210f3d4b";

const suiTransport = new SuiTransport(SUI_RPC_ENDPOINT, RACE_PACKAGE_ID);
const appHelper = new AppHelper(suiTransport);

console.log('AppHelper is connected to Sui!');
```

***

## Game Management

These methods cover the lifecycle of a game, from creation and discovery to its conclusion.

### **Creating a Game (`createGame`)**

This method creates a new game account on the blockchain. It returns a `ResponseStream` (covered in [Core Concepts](core-concepts.md)) that allows you to track the transaction's lifecycle.

### **Parameters (`CreateGameAccountParams`):**

* `title`: The public name of the game (max 16 characters).
* `bundleAddr`: The on-chain address of the `GameBundle` containing the game's logic.
* `tokenAddr`: The address of the SPL token or native currency used for deposits.
* `maxPlayers`: The maximum number of players that can join.
* `entryType`: An object defining the entry conditions.
  * Example for a cash game: `{ kind: 'cash', minDeposit: 1000n, maxDeposit: 10000n }`
  * Example for a ticket game: `{ kind: 'ticket', amount: 100n }`
* `registrationAddr`: The address of a public registry where the game will be listed.
* `recipientAddr`: The address of the RecipientAccount that will collect fees and winnings.
* `data`: A `Uint8Array` of initial data to configure the game state.

```typescript
import { FacadeWallet } from '@race-foundation/sdk-facade';

const myWallet = new FacadeWallet();

const gameParams = {
    title: 'My First Game',
    bundleAddr: 'facade-bundle-address',
    tokenAddr: 'FACADE_NATIVE',
    maxPlayers: 8,
    entryType: { kind: 'cash' as const, minDeposit: 1000n, maxDeposit: 10000n },
    registrationAddr: 'default-registration',
    recipientAddr: 'facade-recipient-address',
    data: new Uint8Array([1, 2, 3]), // Initial game state data
};

const responseStream = appHelper.createGame(myWallet, gameParams);

// Handle the response stream to track transaction progress
(async () => {
    for await (const response of responseStream) {
        if (!response) break;
        console.log('Transaction status:', response.status);
        if (response.status === 'succeed') {
            console.log('Game created at address:', response.data.gameAddr);
            console.log('Signature:', response.data.signature);
        } else if (response.status === 'failed') {
            console.error('Failed to create game:', response.error);
        }
    }
})();
```

### **Fetching Games (`getGame` and `listGames`)**

You can fetch the on-chain data for a single game or a list of games from a registration account.

```typescript
// Fetch a single game by its address
const gameAccount = await appHelper.getGame('GAME_ADDRESS_HERE');

if (gameAccount) {
    console.log(`Game Title: ${gameAccount.title}`);
    console.log(`Players Joined: ${gameAccount.players.length}/${gameAccount.maxPlayers}`);
}

// Fetch all games listed in a registration account
const allGames = await appHelper.listGames(['REGISTRY_ADDRESS_HERE']);
console.log(`Found ${allGames.length} games.`);
```

### **Registering and Closing a Game**

* **`registerGame(wallet, gameAddr, regAddr)`**: Adds an existing game to a registration list to make it discoverable.
* **`closeGame(wallet, regAddr, gameAddr)`**: Closes a game account. This is typically done by the owner after a game has concluded to manage on-chain assets and reclaim rent.

***

## Player Actions

These methods are used to manage player identity and game entry.

### **Creating a Player Profile (`createProfile`)**

A player profile is a player's on-chain identity, including a nickname and an optional PFP NFT. This is often required before joining a game.

````typescript
const myWallet = new FacadeWallet();

const profileStream = appHelper.createProfile(
    myWallet,
    'PlayerOne',
    'nft01' // Optional PFP NFT address
);

(async () => {
    for await (const response of profileStream) {
        if (!response) break;
        console.log('Profile creation status:', response.status);
        if (response.status === 'succeed') {
            console.log('Profile created!', response.data.profile);
        }
    }
})();```

### **Joining a Game (`join`)**

This method allows a player to join a game, potentially creating their profile in the same transaction if needed.

**Parameters (`JoinOpts`):**

*   `addr`: The address of the game to join.
*   `amount`: The deposit or ticket amount required for entry.
*   `keys`: The player's public encryption keys, obtained from an `Encryptor` instance.
*   `createProfileIfNeeded` (optional): If true, the SDK will create a default profile for the player if one doesn't exist.

```typescript
import { Encryptor } from '@race-foundation/sdk-core';

const myWallet = new FacadeWallet();

// In a real app, you would create and persist the encryptor for the user.
const encryptor = await Encryptor.create(myWallet.walletAddr);
const playerKeys = await encryptor.exportPublicKey();

const joinStream = appHelper.join(myWallet, {
    addr: 'GAME_ADDRESS_HERE',
    amount: 5000n, // The deposit amount
    keys: playerKeys,
    createProfileIfNeeded: true,
});

// Handle the response stream to track progress
(async () => {
    for await (const response of joinStream) {
        if (!response) break;
        if (response.status === 'succeed') {
            console.log('Successfully joined the game! Signature:', response.data.signature);
        }
    }
})();
````

### **Making a Deposit (`deposit`)**

For games that allow it, players can add more funds after joining.

```typescript
const depositStream = appHelper.deposit(myWallet, {
    addr: 'GAME_ADDRESS_HERE',
    amount: 2500n, // Additional deposit amount
});
// ... handle response stream
```

***

## Interacting with Assets

The `AppHelper` also includes utilities for querying and managing on-chain assets related to the RACE ecosystem.

### **Listing Tokens and NFTs (`listTokens`, `listNfts`)**

These methods query a wallet's holdings. You can optionally provide a `Storage` implementation to cache the results and improve performance.

```typescript
const walletAddress = myWallet.walletAddr;

// List all NFTs owned by the wallet
const nfts = await appHelper.listNfts(walletAddress);
console.log(`Found ${nfts.length} NFTs.`);

// List specific tokens and their balances
const tokenAddresses = ['FACADE_USDC', 'FACADE_RACE'];
const tokenBalances = await appHelper.listTokenBalance(walletAddress, tokenAddresses);

for (const balance of tokenBalances) {
    console.log(`Token: ${balance.addr}, Amount: ${balance.amount}`);
}
```

### **Managing Game Bonuses and Rewards (`attachBonus`, `claim`)**

* **`attachBonus(wallet, gameAddr, bonuses)`**: Allows a game owner or sponsor to attach additional token prizes to a game. bonuses is an array of `{ identifier, tokenAddr, amount }`.
* **`previewClaim(wallet, recipientAddr)`**: A read-only method to see what funds are available for a recipient to claim from a game's fee/prize pool.
* **`claim(wallet, recipientAddr)`**: Executes the transaction to withdraw claimable funds to the recipient's wallet.

```typescript
const recipientAddress = 'RECIPIENT_ADDRESS_HERE';

// First, preview what can be claimed
const claims = await appHelper.previewClaim(myWallet, recipientAddress);
if (claims.length > 0) {
    console.log('Claimable amounts:', claims);
    
    // If there's something to claim, execute the transaction
    const claimStream = appHelper.claim(myWallet, recipientAddress);
    
    (async () => {
        for await (const response of claimStream) {
            if (!response) break;
            if (response.status === 'succeed') {
                console.log('Successfully claimed rewards!');
            }
        }
    })();

} else {
    console.log('Nothing to claim at the moment.');
}
```
