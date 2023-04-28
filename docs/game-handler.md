# Game Handler

On Race, the game core is written in Rust and compiled to WebAssembly.  All it needs is a game handler, a simple state machine, described as below:

```rust
trait GameHandler {
  init_state(effect: Effect, init_account: InitAccount) -> Result<Self>;

  handle_event(&mut self, effect: Effect, event: Event) -> Result<()>;
}
```

## Initialization
The game handler is initialized with the state of an on-chain account, which stores the game data such as the players, servers and so on.  Once the game handler is created, it's ready to receive and handle events.

## Effect
The game handler should be pure: no randomization nor clock access.  The calculation remains the same with the same input.  We use Effect to ship all necessary information from execution runtime. Additionally, game handler uses Effect to acquire randomness, assign secrets and reveal hidden knowledge.  This model makes it easy to test and simulate.

Here are the context inforamtion provided by effect:
- Event timestamp
- Number of players and servers
- Revealed hidden knowledge


## Event Handling
The game progress is driven by handling events, Race provides a list of predefined types of event, with each automatically handled by the protocol.  Developers only need to deal with those of interest.  Each game has its own specific events which are defined separately in the game handler and will be transferred to CustomEvent.

Here is the list (or table?) of builtin event types:

- `Custom`: a specific game event
- `GameStart`: the game is started
- `GameStart`: the update of on-chain account, with new players and servers
- `GameStart`: a player has left
- `ServerLeave`: a server has left
- `ActionTimeout`: a waiting timeout for player action
- `WaitingTimeout`: a general waiting timeout used for any purposes
- `OperationTimeout`: a situation where the server is probably disconnected
- `Mask`: an event for randomization
- `Lock`: an event for randomization
- `ShareSecrets`: an event for assignment or revealing a hidden information
- `RandomnessReady`: indicates the randomness is generated
- `SecretsReady`: the secrets shared and revealed information accessible
- `Shutdown`: the game is over

### TODO
- [ ] an example to show what events are usually or must be included in a game
- [ ] an flowchart to illustrate the above example
