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
race-core = { path = "../../core" }
race-proc-macro = { path = "../../proc-macro" }
serde_json = "1.0.85"
serde = "1.0.144"
borsh = "0.9.3"
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

#[derive(BorshDeserialize, BorshSerialize)]
#[game_handler]
struct Raffle {
    previous_winner: Option<String>,
    random_id: RandomId,
    next_draw: u64,
    players: Vec<Player>,
}
```

Firstly, we need a vector of `Player` to represent all participants.
A game handler must track the player status itself.  Here, only
address and balance is required for a player state.

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
use race::core::prelude::*;

impl GameHandler for Raffle {
    ...
}
```

To make our `Raffle` a game handler, two functions must be implemented:

- `init_state`, is called when the game is loaded.
- `handle_event`, is called each time a message is received.

### init_state

```rust
impl GameHandler for Raffle {
    fn init_state(context: &mut Effect, init_account: GameAccount) -> Result<Self> {
        let players = init_account
            .players
            .into_iter()
            .map(|p| Player {
                addr: p.addr,
                balance: p.balance,
            })
            .collect();

        let draw_time = context.timestamp() + 30_000;

        Ok(Self {
            players,
            random_id: 0,
            draw_time,
        })
    }

    ...
}
```

The implementation is pretty straight forward, we take the players
information from account, and set a draw time at 30 seconds later.

### handle_event

```rust
impl GameHandler for Raffle {
    ...

    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least one player to start, otherwise we will skip this draw.
                if context.count_players() >= 1 {
                    let options = self.players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = RandomSpec::shuffled_list(options);
                    self.random_id = context.init_random_state(rnd_spec);
                } else {
                    self.draw_time = context.timestamp() + 30_000;
                    context.wait_timeout(30_000);
                }
            }

            Event::Sync { new_players, .. } => {
                let players = new_players.into_iter().map(Into::into);
                self.players.extend(players);
            }

            // Reveal the first address when randomness is ready.
            Event::RandomnessReady { .. } => {
                context.reveal(self.random_id, vec![0]);
            }

            // Start game when we have enough players.
            Event::WaitingTimeout => {
                context.start_game();
            }

            // Eject all players when encryption failed.
            Event::OperationTimeout { .. } => {
                context.wait_timeout(60_000);
                self.cleanup();
            }

            Event::SecretsReady => {
                let winner = context
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .to_owned();

                for p in self.players.iter() {
                    if p.addr.ne(&winner) {
                        context.settle(Settle::add(&winner, p.balance));
                        context.settle(Settle::sub(&p.addr, p.balance));
                    }
                    context.settle(Settle::eject(&p.addr));
                }
                context.wait_timeout(5_000);
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
will receive, but we only deal the ones we care about.


### Testing

The game handler itself is testable. Let's test some simple cases.

```rust

```

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
