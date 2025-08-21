# ⚙️ Low-Level API: The AppClient

The `AppClient` is the powerhouse of the RACE SDK, designed for building rich, real-time, and stateful applications. Unlike the stateless `AppHelper`, which is excellent for one-off actions, the `AppClient` maintains a persistent connection to a game's transactor, allowing your application to receive a live stream of game events and updates.

***

## When to Use `AppClient`

You should use the `AppClient` when you need to:

* **Maintain a persistent game session:** Keep the user connected to a game instance for an extended period.
* **Receive real-time updates:** Automatically receive and react to game events as they happen without needing to poll for changes.
* **Build interactive UIs:** Create dynamic user interfaces that reflect the current game state accurately.
* **Handle complex game logic:** Manage applications where the state changes frequently based on multiple players' actions.

**Common Use Cases:**

* Turn-based or real-time multiplayer games.
* Live auction or bidding platforms.
* Decentralized chat applications tied to a game.
* Any application that requires a live, interactive connection to a shared state machine.

***

## Initialization and Connection

Initializing the `AppClient` is the first step to establishing a live game session. The static `initialize` method fetches all necessary on-chain data and prepares the client for connection.

### **Initialization**

The `AppClient.initialize(opts)` method orchestrates the entire setup process. It fetches the `GameAccount`, the `GameBundle` (WASM code), and the transactor's server details, then sets up the cryptographic layer for the player.

```typescript
import { AppClient, IStorage, Storage } from '@race-foundation/sdk-core';
import { SolanaTransport } from '@race-foundation/sdk-solana'; // Or SuiTransport, FacadeTransport

// 1. Set up the transport and an optional storage provider
const transport = new SolanaTransport('solana:mainnet', 'https://api.mainnet-beta.solana.com');
const storage: IStorage = new Storage(); // Uses IndexedDB to cache keys, profiles, etc.

async function connectToGame(gameAddress: string, playerAddress: string) {
  try {
    // 2. Initialize the AppClient with all necessary parameters and callbacks
    const appClient = await AppClient.initialize({
      transport,
      storage, // Recommended for persisting encryption keys
      gameAddr: gameAddress,
      playerAddr: playerAddress,
      onEvent: handleGameEvent,
      onConnectionState: handleConnectionState,
      onError: handleSdkError,
      onReady: (context, state) => {
        console.log('Game is ready! Initial state:', state);
        // You can now enable UI elements for the user
      },
    });

    // 3. Once initialized, attach to the game to start the WebSocket connection and receive events.
    await appClient.attachGame();
    
    return appClient;

  } catch (error) {
    console.error('Failed to initialize AppClient:', error);
  }
}
```

**`AppClientInitOpts` Breakdown:**

* `transport`: An instance of a transport layer (e.g., `SolanaTransport`, `SuiTransport`).
* `storage` (optional): An implementation of `IStorage` used to cache the player's encryption keys, profiles, and other data to improve performance and user experience. The SDK exports a default `Storage` class using IndexedDB.
* `gameAddr`: The public key (address) of the `GameAccount` you want to connect to.
* `playerAddr`: The public key of the current user.
* `onEvent`: **(Required)** The primary callback function that is triggered for every `GameEvent`, delivering state updates.
* `onMessage` (optional): Callback for handling in-game chat messages.
* `onTxState` (optional): Callback for receiving updates on transaction states.
* `onConnectionState` (optional): Callback to monitor the WebSocket connection status (`connected`, `disconnected`, etc.).
* `onError` (optional): Callback for handling SDK-specific errors, such as a state mismatch or failure to attach.
* `onReady` (optional): A crucial callback that fires once the client is initialized, has processed all backlog events, and is fully synchronized with the current game state. This is the ideal moment to render the main game UI.
* `onProfile` (optional): A callback that fires whenever a player profile (with PFP) is loaded or updated.

### **Attaching to the Game**

After a successful initialization, `appClient.attachGame()` must be called. This method:

1. Connects to the game's transactor via a WebSocket.
2. Subscribes to the event stream for the specified game.
3. Receives and processes any historical (backlog) events to bring the client up to the current state.
4. Calls your `onReady` callback once the client is fully synced.

### **Handling Connection States**

You can monitor the connection's health using the `onConnectionState` callback. This is useful for providing feedback to the user (e.g., showing a "Reconnecting..." message).

```typescript
import { ConnectionState } from '@race-foundation/sdk-core';

function handleConnectionState(state: ConnectionState) {
  console.log('Connection state changed:', state);
  // Example: Update your UI based on the connection status
  if (state === 'reconnected') {
    // Show a "Reconnected!" message
  } else if (state === 'disconnected') {
    // Show a "Connection lost, attempting to reconnect..." message
  }
}
```

***

## Real-time Event Handling

The core of a real-time application built with `AppClient` is the onEvent callback. It's the central point where your application receives updates and new game states.

```typescript
import { GameContextSnapshot, GameEvent, EventCallbackOptions } from '@race-foundation/sdk-core';

function handleGameEvent(
  context: GameContextSnapshot,
  state: Uint8Array,
  event: GameEvent,
  options: EventCallbackOptions
) {
  console.log('Received game event:', event.kind());
  console.log('New game context snapshot:', context);
  console.log('Is this a checkpoint event?', options.isCheckpoint);

  // This is where you update your application's state and re-render the UI.
  // For example, in a React app:
  // setGameContext(context);
}
}
```

#### **Callback Parameters Explained:**

* `context: GameContextSnapshot`: An **immutable snapshot** of the current game state. It provides high-level, easy-to-access information like the list of players (`context.nodes`), game status, and revealed secrets. See Section 6.5 for more details.
* `state: Uint8Array`: The raw, serialized game state from the game's WASM module. This can be used for custom deserialization if your game logic requires it.
* `event: GameEvent`: The specific event that caused this state update. You can use `event.kind()` to determine the type of event and react accordingly (e.g., play an animation for an `Attack` event).
* `options: EventCallbackOptions`: Additional metadata about the event, such as `isCheckpoint`, which tells you if this state change was part of a major, on-chain verifiable checkpoint.

***

## Submitting Player Actions

Players interact with the game by submitting events. This is done primarily through the `submitEvent` method.

### **Defining Custom Events**

First, define your game's specific actions by creating classes that implement the `ICustomEvent` interface. You must use the `@race-foundation/borsh` package to define the serialization schema.

```typescript
import { field, serialize, ICustomEvent } from '@race-foundation/borsh';

class PlaceBet implements ICustomEvent {
  @field('u64')
  amount: bigint;

  constructor(fields: { amount: bigint }) {
    this.amount = fields.amount;
  }

  serialize(): Uint8Array {
    return serialize(this);
  }
```

### **Submitting an Event**

Once you have an initialized `appClient`, you can create an instance of your custom event and submit it. The SDK automatically wraps it in a `Custom` event, signs it, and sends it to the transactor.

```typescript
async function submitPlayerBet(appClient: AppClient, betAmount: bigint) {
  try {
    const betEvent = new PlaceBet({ amount: betAmount });
    await appClient.submitEvent(betEvent);
    console.log('Bet submitted successfully!');
  } catch (error) {
    console.error('Failed to submit bet:', error);
  }
}
```

### **Other Actions**

* `submitMessage(content: string)`: Sends a simple string message, handled by the `onMessage` callback. Useful for in-game chat.
* `exit(keepConnection: boolean = false)`: Allows the current player to leave the game. If `keepConnection` is `false`, the WebSocket connection will be terminated.
* `detach()`: Disconnects the client from the event stream without making the player leave the game. The session can be resumed later by calling `attachGame()` again.

***

## Understanding the `GameContextSnapshot`

The `GameContextSnapshot` object is an immutable, easy-to-use representation of the game state at a specific moment. It is passed to your `onEvent` and `onReady` callbacks and is the primary source of data for rendering your UI.

### **Key Properties of `GameContextSnapshot`:**

* `gameAddr`: The address of the game.
* `status`: The current status of the game (`'idle'`, `'running'`, or `'closed'`).
* `nodes`: An array of `NodeSnapshot` objects, representing all participants (players and servers), their addresses, IDs, and connection statuses.
* `revealed`: A map containing decrypted secrets from the game's multi-party computation randomness protocol.

### **Accessing Revealed Secrets**

Many games involve hidden information that is revealed over time (e.g., cards in a poker game). The `Encryptor` handles the complex decryption process automatically. The revealed data can be accessed directly from the `GameContextSnapshot`.

```typescript
function handleGameEvent(context: GameContextSnapshot, /* ... */) {
  // `revealed` is a Map where the key is the `randomId` (identifying a specific randomness event)
  // and the value is another Map of the secret's index to its revealed string value.
  for (const [randomId, secrets] of context.revealed.entries()) {
    console.log(`Revealed secrets for Random ID ${randomId}:`);
    for (const [index, value] of secrets.entries()) {
      console.log(`  Index ${index}: ${value}`);
      // Example: If index 0 is a card, you might do: `displayCard(value)`
    }
  }
}

// You can also use the getRevealed(randomId) method on the appClient instance at any time.
const revealedSecretsForRound1 = appClient.getRevealed(1);
```

***

## Handling Sub-Games (`subClient`)

The RACE SDK supports the concept of "sub-games," which are new game instances spawned from a main game. This is useful for complex games with multiple rounds or stages, such as poker tournaments where players move between different tables.

A `SubClient` is used to connect to and interact with a sub-game. It functions almost identically to an `AppClient` but operates within the scope of its specific sub-game.

You can create a `SubClient` from an existing, initialized `AppClient` when a `LaunchSubGame` effect is emitted.

```typescript
async function connectToSubGame(mainClient: AppClient, gameId: number) {
  try {
    const subClient = await mainClient.subClient({
      gameId,
      onEvent: handleSubGameEvent, // A separate callback for the sub-game's events
      onReady: (context, state) => {
        console.log(`Sub-game #${gameId} is ready!`);
        // Render the sub-game UI
      }
      // ... other optional callbacks
    });

    await subClient.attachGame();
    console.log(`Successfully attached to sub-game #${gameId}`);
    return subClient;

  } catch (error) {
    console.error(`Failed to create sub-client for game ${gameId}:`, error);
  }
}
```

The `subClient` will have its own isolated `GameContext` but will share the underlying WebSocket connection and encryption layers with its parent `AppClient`, making it an efficient way to manage complex, multi-stage game flows.
