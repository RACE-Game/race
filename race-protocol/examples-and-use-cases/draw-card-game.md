# Draw Card Game

The Draw Card game is a minimal poker-like game designed to showcase the basic principles of Race Protocol. It demonstrates how to implement core functionalities like player interactions, randomness generation, and settlements.

### Game Overview

Here's a brief overview of the Draw Card game:

* **Players**: Two players participate in each game.
* **Dealing**: Each player receives one random card from a standard deck of 52 cards.
* **Betting**: The first player to act places a bet within the predefined minimum and maximum bet limits.
* **Reacting**: The second player can either call the bet or fold.
* **Revealing**: If the second player calls, both players reveal their cards.
* **Outcome**: The player with the higher card wins the pot. If both players have the same card, the second player wins. If the second player folds, the first player wins the pot.
* **Settlements**: After the game concludes, the Transactor server handles the settlement process, distributing the pot to the winner and updating player balances accordingly.

### Implementation Walkthrough

The Draw Card game logic is implemented in the `race-example-draw-card` crate. Here's a detailed walkthrough of the implementation:

1. **Defining Game State**:
   1. The `DrawCard` struct represents the game state, including:
      1. `last_winner`: An optional string indicating the address of the previous game's winner.
      2. `random_id`: The identifier of the randomness used for dealing cards.
      3. `players`: A vector of Player structs, each containing the player's ID, balance, and current bet.
      4. `stage`: The current stage of the game (e.g., Dealing, Betting, Reacting, Revealing, Ending).
      5. `pot`: The total amount of tokens in the pot.
      6. `bet`: The current bet amount placed by the first player.
      7. `blind_bet`, `min_bet`, `max_bet`: Game parameters defining the blind bet, minimum bet, and maximum bet allowed
2. **Defining Custom Events**:
   1. The `GameEvent` enum defines the custom events used in the game:
      1. `Bet(u64)`: Represents a player placing a bet.
      2. `Call`: Represents the second player calling the bet.
      3. `Fold`: Represents the second player folding.
3. **Implementing GameHandler**:
   1. The `DrawCard` struct implements the GameHandler trait, providing the following functions:
      1. `init_state`: Initializes the game state based on the on-chain game account data.
      2. `handle_event`: Processes incoming events and updates the game state accordingly.
      3. `into_checkpoint`: Converts the game state into a checkpoint object for on-chain storage.
4. **Handling Events**:
   1. The `handle_event` function processes different event types:
      1. `Custom Events`: These events are defined in the GameEvent enum and represent player actions like betting, calling, and folding. The handler validates the actions based on the current game stage and updates the state accordingly.
      2. `Built-in Events`: The handler responds to built-in events like `GameStart`, `Join`, `RandomnessReady`, and `SecretsReady` to manage the game flow, generate randomness, assign cards, reveal information, and trigger settlements.
5. **Randomization and Decryption**:
   1. The game handler uses the `Effect` object to request the creation of a shuffled list of cards. The Transactor and validator servers handle the randomization process and share the necessary secrets with the handler.
   2. When the `SecretsReady` event is received, the handler uses the shared secrets to decrypt the assigned cards and determine the winner.
6. **Settlements**:
   1. After determining the winner, the handler uses the `Effect` object to specify the asset distribution for the settlement transaction. The Transactor then submits the transaction to the blockchain, updating player balances and finalizing the game.

### Testing and Simulation

The `race-test` crate provides tools for unit and integration testing of the Draw Card game logic. Developers can simulate player actions, server interactions, and blockchain updates to verify that the game functions correctly and fairly.

This walkthrough demonstrates how Race Protocol simplifies the development of blockchain games by providing a structured framework for managing game state, handling events, and interacting with the blockchain. By building upon these basic principles, developers can create more complex and engaging games with diverse mechanics and functionalities.

\
