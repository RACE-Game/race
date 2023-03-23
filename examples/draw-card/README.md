# Draw Card

In this tutorial, we will build a minimal poker game for only two
players, each game has one player in turn as dealer.  Both of them
will get one private card as hands.  Then the dealer decides an amount
to bet.  Another player can either call or fold. If he calls, both
players will reveal their hands, the one with better hands will win.
If he fold, the dealer player will win.

## Set Up the Project

```shell
# Create a lib project with Cargo.
cargo new my-draw-card --lib
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

## Define On-chain Game Properties

## Implement GameHandler Trait

### init_state

### handle_event

### Testing

### Summary

That's it! You can find the completed version at [https://github.com/RACE-Game/race/tree/master/examples/draw-card].
