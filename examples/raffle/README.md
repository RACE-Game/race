# Raffle

In this tutorial, we will build a raffle game that draws every 30
seconds.  Player can join with tokens, and each token will grant the
player one ticket.

## Set Up the Project

```shell
# Create a lib project with Cargo.
cargo new my-raffle --lib
```

Edit `Cargo.toml` to add:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
race-core = "0.0.1"
```

Now, we are going to write the game logic.  Open `src/lib.rs` in your code editor:

## Define Game State

```rust
use race::core::prelude::*;

#[derive(BorshDeserialize, BorshSerialize)]
struct Player {
    pub addr: String,
    pub balance: u64,
}

impl From<PlayerJoin> for Player {
    fn from(value: PlayerJoin) -> Self {
        Self {
            addr: value.addr,
            balance: value.balance,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[game_handler]
struct Raffle {
    last_winner: Option<String>,
    players: Vec<Player>,
    random_id: RandomId,
    draw_time: u64,
}
```

Firstly, we need a vector of `Player` to represent all participants.
A game handler must track the player status. For each player, we save its
address and balance.

Secondly, we need to track all random ids used in the game.  In a
raffle, we only need one randomness, saved as `random_id`.  The zero
value represents "not ready".

Last, we need to track the time for draw as `draw_time`, we use `u64` for timestamps.

The struct must derive `BorshDeserialize` and `BorshSerialize`, so
that they can be passed between runtime and wasm.  The macro
`game_handler` is used to generate boilerplate code for
serialization/deserialization.

## Implement GameHandler Trait

```rust
impl GameHandler for Raffle {
    ...
}
```

To make our `Raffle` a game handler, two functions must be implemented:

- `init_state`, is called when the game is loaded.
- `handle_event`, is called each time an event is received.

### init_state

```rust
impl GameHandler for Raffle {
    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self> {
        let players = init_account.players.into_iter().map(Into::into).collect();
        let draw_time = 0;
        Ok(Self {
            last_winner: None,
            players,
            random_id: 0,
            draw_time,
        })
    }

    ...
}
```

The `Effect` is the bridge between runtime and wasm. The `InitAccount` is the onchain game account snapshot. In `init_state`, we initialize the game state, and do necessary validation.

The implementation is pretty straight forward, we take the players information from account.

### handle_event

```rust
const DRAW_TIMEOUT: u64 = 30_000;

impl GameHandler for Raffle {
    ...

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least one player to start, otherwise we will skip this draw.
                if effect.count_players() >= 1 {
                    let options = self.players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = RandomSpec::shuffled_list(options);
                    self.random_id = effect.init_random_state(rnd_spec);
                }
            }

            Event::Sync { new_players, .. } => {
                let players = new_players.into_iter().map(Into::into);
                self.players.extend(players);
                if self.players.len() >= 1 && self.draw_time == 0 {
                    self.draw_time = effect.timestamp() + DRAW_TIMEOUT;
                    effect.wait_timeout(DRAW_TIMEOUT);
                }
            }

            // Reveal the first address when randomness is ready.
            Event::RandomnessReady { .. } => {
                effect.reveal(self.random_id, vec![0]);
            }

            // Start game when we have enough players.
            Event::WaitingTimeout => {
                if self.players.len() >= 1 {
                    effect.start_game();
                }
            }

            // Eject all players when encryption failed.
            Event::OperationTimeout { .. } => {
                self.cleanup();
            }

            Event::SecretsReady => {
                let winner = effect
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .to_owned();

                for p in self.players.iter() {
                    if p.addr.ne(&winner) {
                        effect.settle(Settle::add(&winner, p.balance));
                        effect.settle(Settle::sub(&p.addr, p.balance));
                    }
                    effect.settle(Settle::eject(&p.addr));
                }
                self.last_winner = Some(winner);
                self.cleanup();
            }
            _ => (),
        }
        Ok(())
    }
}
```

Here we are at the most difficult part.  There are a lot of events we
will receive, but we only deal those we care. Following is the list of relevant events in the order of occurrence.

- Sync: We receive this event after a new player or a server joined. We want to schedule the game start when we have players in game, so we dispatch `WaitingTimeout` event.
- WaitingTimeout: Check if we have enough players, then start the game.
- GameStart: We receive this event when game is started. We must make sure that we have at least 1 player in the game. During the start up, we create one shuffled list of player addresses, and save its random id in game state.
- RandomnessReady: We receive this event when the randomness is ready, which means all shuffling and encryption is done. We simply reveal the first item as the winner address.
- SecretsReady: We receive this event when secrets are shared, which means we are able to check the value of first item now. We take the winner address, and generate a list of settles as game result.

### Testing

The game handler itself is testable. Let's test some simple cases.

#### Unit Tests

We can write unit tests for each event.

- Initiate `Effect`, `Event` and `State`.
- Call `handle_event`.
- Do assertion for updated `Effect` and `State`.

```rust
#[test]
fn test_sync() {
    let mut effect = Effect::default();
    let mut state = Raffle {
        draw_time: 0,
        last_winner: None,
        players: vec![],
        random_id: 0,
    };
    let event = Event::Sync {
        new_players: vec![PlayerJoin {
            addr: "alice".into(),
            position: 0,
            balance: 100,
            access_version: 0,
            verify_key: "".into(),
        }],
        new_servers: vec![ServerJoin {
            addr: "foo".into(),
            endpoint: "foo.endpoint".into(),
            access_version: 0,
            verify_key: "".into(),
        }],
        transactor_addr: "".into(),
        access_version: 0,
    };

    state.handle_event(&mut effect, event).unwrap();
    assert_eq!(state.players.len(), 1);
    assert_eq!(effect.wait_timeout, Some(DRAW_TIMEOUT));
}
```

### Integration Tests

TBD


### Summary

The game logic is built as a game handler, which holds the data of the
game, and implements `init_state` and `handle_event`.  To generate a
randomness, we use `Effect::init_random_state`.  The randomness will
not be generated in game handler, instead it's generated by the
servers running behind.  When the randomness is ready, we can reavel
it with `Effect::reveal`. Then, when the secrets are shared, we can
check the value with `Effect::get_revealed`.  In the end of the game,
we use `Effect::settle` to make on-chain settlements.

That's it! You can find the completed version at [https://github.com/RACE-Game/race/tree/master/examples/raffle].
