---
description: >-
  This guide will walk you through the initial steps of setting up your
  development environment, installing the RACE SDK, and running your first piece
  of code to interact with a game.
---

# 🚀 Getting Started

## **Installation**

The RACE SDK is a modular collection of packages hosted on NPM. You should install only the packages relevant to your project's needs. All packages are scoped under `@race-foundation`.

### **Core Package**

At a minimum, every project will need the core SDK package. It contains the essential, blockchain-agnostic logic, interfaces, and classes.

```bash
npm install @race-foundation/sdk-core
```

### **Blockchain-Specific Packages**

Depending on your target blockchain, you must install the corresponding transport package along with its required peerDependencies.

**For Solana:**

````bash
npm install @race-foundation/sdk-solana @solana/web3.js @solana-program/token @solana-program/system```

**For Sui:**

```bash
npm install @race-foundation/sdk-sui @mysten/sui @suiet/wallet-sdk
````

**Facade Package (for Development & Testing)**

For local development, prototyping, or getting started without connecting to a live blockchain, we provide a facade package. It simulates the blockchain transport layer, allowing you to build and test your application logic quickly.

```bash
npm install @race-foundation/sdk-facade
```

**Borsh Serialization**

The SDK uses the Borsh serialization format for all on-chain data structures. Our custom implementation, which uses decorators for easy schema definition, is available in its own package. You will need this if you are defining custom game states or events.

```bash
npm install @race-foundation/borsh
```

***

## **Environment Setup (tsconfig.json)**

The RACE SDK is written in TypeScript and uses modern features, including decorators for Borsh serialization. Your project's `tsconfig.json` file must be configured to support these features.

Create a tsconfig.json file in your project's root with the following configuration:

```json
{
  "compilerOptions": {
    "target": "es2020",
    "module": "commonjs",
    "moduleResolution": "node",
    "esModuleInterop": true,
    "experimentalDecorators": true,
    "forceConsistentCasingInFileNames": true,
    "strict": true,
    "skipLibCheck": true
  }
}
```

**Key Options Explained:**

* `"target": "es2020"`: This is required to support modern JavaScript features like bigint, which the SDK uses extensively for handling on-chain numerical values.
* `"experimentalDecorators": true`: **This is essential.** The `@race-foundation/borsh` package relies on decorators (e.g., `@field`) to define serialization schemas. Your project will not compile without this flag enabled.
* `"moduleResolution": "node"`: Ensures that Node.js-style module resolution is used, which is standard for most TypeScript projects in the ecosystem.

***

## **Quick Start: Your First Interaction**

This tutorial will guide you through connecting to a simulated game using the `sdk-facade`, fetching its state, and joining it. This approach allows you to understand the SDK's core workflow without needing a real wallet or connecting to a live network.

### **Step 1: Set up your project**

First, create a new Node.js project and install the necessary core and facade packages.

```bash
mkdir race-quickstart
cd race-quickstart
npm init -y
npm install @race-foundation/sdk-core @race-foundation/sdk-facade typescript ts-node
```

### **Step 2: Create your script file**

Create a file named `index.ts` and add the following code. This script will simulate creating a wallet, finding a game, and joining it.

```typescript
import { AppHelper, Encryptor } from '@race-foundation/sdk-core';
import { FacadeTransport, FacadeWallet } from '@race-foundation/sdk-facade';

async function main() {
  console.log('--- Initializing RACE SDK Facade ---');
  
  // 1. Initialize the Facade Transport to simulate a blockchain connection.
  const transport = new FacadeTransport();
  
  // 2. Create an AppHelper, the high-level API for common actions.
  const helper = new AppHelper(transport);

  // 3. Create a simulated wallet for our player.
  const myWallet = new FacadeWallet('my-player-address');
  console.log(`Player wallet created with address: ${myWallet.walletAddr}`);

  // 4. Create an Encryptor to manage cryptographic keys.
  // The `join` method requires public keys, even for the facade.
  const encryptor = await Encryptor.create(myWallet.walletAddr);
  const playerKeys = await encryptor.exportPublicKey();

  // 5. List available games. The facade comes pre-populated with samples.
  console.log('\n--- Fetching Available Games ---');
  // 'default-registration' is a pre-configured registry in the facade.
  const allGames = await helper.listGames(['default-registration']);
  if (allGames.length === 0) {
    console.log('No games found on the facade.');
    return;
  }
  console.log(`Found ${allGames.length} game(s).`);
  
  // 6. Select the first game and inspect its details.
  const gameToJoin = allGames[0];
  console.log(`\n--- Inspecting Game: "${gameToJoin.title}" (${gameToJoin.addr}) ---`);
  console.log(`Max Players: ${gameToJoin.maxPlayers}`);
  console.log(`Current Players: ${gameToJoin.players.length}`);

  // 7. Join the game. This returns a ResponseStream.
  console.log(`\n--- Joining Game ---`);
  const joinResponseStream = helper.join(myWallet, {
    addr: gameToJoin.addr,
    amount: 1000n, // A mock deposit amount.
    keys: playerKeys, // Provide the generated public keys.
  });

  // 8. Handle the asynchronous response stream to track the transaction's status.
  for await (const response of joinResponseStream) {
    if (!response) break;
    console.log(`Join transaction status: ${response.status}`);
    if (response.status === 'succeed') {
      console.log('Successfully joined the game!');
      console.log('Mock Transaction Signature:', response.data.signature);
    } else if (response.status === 'failed' || response.status === 'transaction-failed') {
      console.error('Failed to join game:', response.error);
    }
  }

  // 9. Verify that you have joined by fetching the game state again.
  const updatedGame = await helper.getGame(gameToJoin.addr);
  console.log(`\n--- Verifying Game State ---`);
  console.log(`Current Players: ${updatedGame?.players.length}`);
  console.log('Players in game:', updatedGame?.players.map(p => p.addr));
}

main().catch(error => {
  console.error('An error occurred:', error);
});
```

### **Step 3: Run the script**

Execute your script from the terminal using `ts-node`

```bash
npx ts-node index.ts
```

**Expected Output:**

You should see a series of console logs detailing each step of the process:

```
--- Initializing RACE SDK Facade ---
Player wallet created with address: my-player-address

--- Fetching Available Games ---
Found 1 game(s).

--- Inspecting Game: "Facade Game" (some-game-address) ---
Max Players: 8
Current Players: 1

--- Joining Game ---
Join transaction status: preparing
Join transaction status: waiting-wallet
Join transaction status: confirming
Join transaction status: succeed
Successfully joined the game!
Mock Transaction Signature: facadesig

--- Verifying Game State ---
Current Players: 2
Players in game: [ 'server-address', 'my-player-address' ]
```

Congratulations! You have successfully used the RACE SDK to find a game, join it, and verify the result, all within a simulated environment. You are now ready to explore more advanced features or integrate the SDK with a live blockchain.
