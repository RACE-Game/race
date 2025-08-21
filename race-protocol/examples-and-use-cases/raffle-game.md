# Raffle Game

The Raffle game is another example implementation that showcases Race Protocol's capabilities for **randomized selection and secure asset distribution**. It demonstrates how to build a simple raffle game where players can join with tokens and have a chance to win the entire pot.

### Game Overview

Here's how the Raffle game works:

* **Players**: Any number of players can participate in the raffle.
* **Joining**: Players join the raffle by depositing tokens into the game account. Each token deposited grants the player one ticket in the raffle.
* **Draw Time**: The raffle has a predefined draw time, after which a winner is randomly selected.
* **Random Selection**: The Transactor server utilizes Race Protocol's P2P randomization process to select a winner from the list of participating players.
* **Settlements**: After the draw, the Transactor handles the settlement process, transferring all tokens in the pot to the winner and ejecting all players from the game.

### Implementation Walkthrough

The Raffle game logic is implemented in the `race-example-raffle` crate. Here's a breakdown of the implementation:

1. Defining Game State:
   1. The `Raffle` struct represents the game state, including:
      1. `last_winner`: An optional integer representing the ID of the previous game's winner.
      2. `players`: A vector of Player structs, each containing the player's ID and balance.
      3. `random_id`: The identifier of the randomness used to select the winner.
      4. `draw_time`: The timestamp when the draw will occur.
2. Implementing GameHandler:
   1. The `Raffle` struct implements the `GameHandler` trait, providing the `init_state` and `handle_event` functions.
3. Handling Events:
   1. The `handle_event` function processes relevant events:
      1. `GameStart`: When the game starts, the handler checks if there are enough players and, if so, creates a shuffled list of player IDs using the Effect object.
      2. `Join`: When a player joins, the handler adds them to the players list and updates the draw\_time if necessary.
      3. `RandomnessReady`: When the randomness is ready, the handler reveals the first item in the shuffled list, which represents the winner's ID.
      4. `SecretsReady`: When secrets are shared, the handler retrieves the revealed winner ID and generates settlements to distribute the pot to the winner and eject all players.
      5. `WaitingTimeout` and `OperationTimeout`: These events are handled to ensure the game progresses smoothly and to clean up the state in case of errors or timeouts.
4. Randomization and Settlements:
   1. The game handler uses the `Effect` object to request the creation of a shuffled list of player IDs. The Transactor and validator servers handle the randomization process and share the necessary secrets with the handler.
   2. After the winner is determined, the handler uses the `Effect` object to specify the settlements, transferring all tokens in the pot to the winner and ejecting all players. The Transactor then submits the settlement transaction to the blockchain.

### Utilizing Randomization and Settlement Features

The Raffle game demonstrates how Race Protocol can be used to:

* **Generate verifiable randomness**: The mental poker algorithm ensures that the winner selection is random, unpredictable, and verifiable by all participants.
* **Securely distribute assets**: The settlement transaction triggered by the game handler ensures that the pot is automatically and securely transferred to the winner without relying on any centralized intermediaries.

This example showcases the power and flexibility of Race Protocol for building games that require secure and transparent randomization and asset distribution mechanisms.

\


\
