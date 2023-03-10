# Game Handler

On Race, the game core is written in Rust, compiled to WebAssembly.  All it needs is a game handler, a simple state machine, can be described as

```rust
trait GameHandler {
  init_state(effect: Effect, init_account: InitAccount) -> Result<Self>;

  handle_event(&mut self, effect: Effect, event: Event) -> Result<()>;
}
```

## Initialization

The game handler can be initialized with the state of an on-chain account, which can tell the players, servers and properties of the game.  Once the game handler is created, it's ready to receive events.

## Effect

The game handler should be pure, no randomization nor no clock access, etc.  The calculation should be always the same with the same input.  We use Effect to ship all necessary information from execution runtime. Additionally, game handler uses Effect to acquire randomness, assign and reveal hidden knowledge, etc.  This model makes it easy to test and simulate.

Here are the context inforamtion provided by effect:
- Event timestamp
- Number of players and servers
- Revealed hidden knowledge

Here are the

## Event Handling

Game progress is driven by handling events, Race provides a list of predefined types of event, each has been already handled by the system.  Developers only deal with those they are interested.  Each game do have specific game events, those are defined separately in the game handler, and transferred as CustomEvent.

Here is the list of builtin event types:

- Custom, represents a game specific events
- GameStart, indicates the game is started
- Sync, the update of on-chain account, with new players and servers
- Leave, indicates a player has been left
- ServerLeave, indicates a server has been left
- ActionTimeout, a waiting timeout for player action
- WaitingTimeout, a general waiting timeout, can be used for any propuse
- OperationTimeout, indicates a server is probably disconnected
- Mask, an event for randomization
- Lock, an event for randomization
- ShareSecrets, an event for assignment or revealing a hidden information
- RandomnessReady, indicates the randomness is generated
- SecretsReady, indicates the secrets have been shared, revealed information is accessible
- Shutdown, indicates the game is over
