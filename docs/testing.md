# Testing

Here we are talking about the testing for game logic, the part written in Rust,compiled to WASM.  The sub project `race-test` provides all the helpers for testing.

## Unit Tests
Since the implementation is a side-effect free handler, it's easy to write unit tests for every event handling cases.

## Integration Tests

It's quite important to have integration tests that go through the whole game progress.  We need to simulates the behavior of different players and servers.  Here is an example where we simulate two players and one servers in a single test.  The code is taken from example `draw-card`.

```rust
// ❶
let account_data = AccountData {
    blind_bet: 100,
    min_bet: 100,
    max_bet: 1000,
};


// ❷
let game_account = TestGameAccountBuilder::default()
    .add_servers(1) // Server names are fixed as Foo, Bar, Baz, etc.
    .add_players(1) // Player names are fixed as Alice, Bob, Charlie, etc.
    .with_data(account_data)
    .build();

let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();

// ❸
let mut alice = TestClient::new("Alice".into(), game_account.addr.clone(), ClientMode::Player);
let mut bob = TestClient::new("Bob".into(), game_account.addr.clone(), ClientMode::Player);

// ❹
let mut transactor = TestClient::new(transactor_addr.clone(), game_account.addr.clone(), ClientMode::Transactor);
```

1. We create a representation of the on-chain game properties, which
   is serialized and stored as `data` in game account.

2. We initialize a fake game account, with the data we created.  We
   added one player to the account, so Alice is already in game now.

3. We create two player test clients, to simulate the player behaviors.

4. We create one server test client, to simulate the server behavior.

The `TestClient` will handle the updated context, and perform all
actions those will be done by the protocol by default, e.g. creating
the randomness, sharing the secrets, etc.  Now, all roles of the game
is prepared, we can start the game.

```rust
let mut ctx = GameContext::try_new(&game_account)?;
let mut handler = TestHandler::init_state(&mut ctx, &game_account)?;
```

We create the game context, and a game handler.  Now it's time to let
the handler receive events from players or servers.  We will create a
sync event, to let Bob join the game as well.

```rust
let av = ctx.get_access_version() + 1;
let sync_event = Event::Sync {
    new_players: vec![PlayerJoin {
        addr: "Bob".into(),
        balance: 10000,
        position: 1,
        access_version: av,
    }],
    new_servers: vec![],
    transactor_addr: transactor_account_addr(),
    access_version: av,
};

handler.handle_event(&mut ctx, &sync_event)?;
```

We use function `TestHandler::handle_event` to process the event.
When it's succeed, the context will be updated.  Sometimes, further
events will be dispatched, that's also recorded in the game context.
In our case, since two players have been joined, a `GameStart` event
will be dispatched.  To let the handler handle the this event, use
function `TestHandler::handle_dispatch_event`.

```rust
handler.handle_dispatch_event(&mut ctx)?;
```

Now the game should be started.

Usually, when a event is handled, all client should be notified with
the update.  For example, in the case of a randomness is created,
servers should shuffle the random items by sending `Mask` and `Lock`
events; in the case of a random item is assigned or revealed, servers
should share their secrets by sending `ShareSecrets`, etc.  We use
function `TestClient::handle_updated_context` to get these generated
events.

```rust
let events: Vec<Event> = transactor.handle_updated_context(&ctx)?;
```

However, it's quite verbose to handle all these system events
manually. In real case, we don't care about system events, they got
handled by the system automatically.  To simulate this behavior, use
function `TestHandler::handle_until_no_events`.  It handles the event,
updates the clients, then handles generated event, and repeat until no
event is dispatching.

```rust
handler.handle_until_no_events(&mut ctx, event, vec![alice, bob, transactor])?;
```

Check example `draw-card` for details.
