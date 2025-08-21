---
description: >-
  This section provides detailed explanations of each instruction available in
  the Race Protocol's Solana program.
icon: sun
---

# Solana Program

The Race Protocol's Solana implementation is a single, comprehensive smart contract that exposes a set of instructions for managing the entire lifecycle of a game. Developers interact with this program by constructing transactions that call these instructions.

The definitive list of instructions can be found in the `RaceInstruction` enum in `race-solana-master/src/instruction.rs`.

### Instruction Set

The table below details each available instruction, its purpose, and typical usage context.

| Instruction                   | Description                                                                                                                                                                                                     | Typical Sender                   |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| **Game Management**           |                                                                                                                                                                                                                 |                                  |
| `CreateGameAccount`           | Initializes a new `GameAccount` on-chain, setting its core properties like the game bundle, token type, max players, and initial data.                                                                          | Game Host / dApp Backend         |
| `CloseGameAccount`            | Closes an empty game account to reclaim SOL from rent. All remaining assets in the stake and bonus accounts are transferred to the game owner.                                                                  | Game Owner                       |
| `RegisterGame`                | Lists an existing `GameAccount` in a `RegistrationAccount`, making it discoverable in a game lobby.                                                                                                             | Game Owner                       |
| `UnregisterGame`              | Removes a `GameAccount` from a `RegistrationAccount`.                                                                                                                                                           | Game Owner / Registry Owner      |
| `Settle`                      | Finalizes a game round. This is the most complex instruction, processing player payouts, ejections, transfers to recipients (rake), and awarding bonuses. It writes a new state checkpoint to the game account. | Transactor Node                  |
| `RejectDeposits`              | Rejects pending player deposits that cannot be accepted by the game logic, refunding the assets to the players.                                                                                                 | Transactor Node                  |
| **Player Management**         |                                                                                                                                                                                                                 |                                  |
| `CreatePlayerProfile`         | Creates a `PlayerProfile` account tied to a player's wallet, storing their nickname and optional PFP. This is required to join games.                                                                           | Player                           |
| `JoinGame`                    | Allows a player to join a game by transferring the required entry fee (`Cash` or `Ticket`) into the game's stake account.                                                                                       | Player                           |
| `Deposit`                     | Allows a player who is already in a game to deposit additional funds, typically for re-buys or add-ons in cash games.                                                                                           | Player                           |
| **Server Management**         |                                                                                                                                                                                                                 |                                  |
| `RegisterServer`              | Creates or updates a `ServerAccount` on-chain, registering a server's public endpoint and associating it with the owner's wallet.                                                                               | Server Operator                  |
| `ServeGame`                   | Allows a registered server to join a game. The first server to join becomes the `Transactor`.                                                                                                                   | Server Operator                  |
| `Vote`                        | Allows a player or server to cast a vote against a potentially unresponsive `Transactor`. An accumulation of votes can halt the game.                                                                           | Player / Validator Node          |
| **Asset & Bundle Management** |                                                                                                                                                                                                                 |                                  |
| `CreateRegistry`              | Creates a new `RegistrationAccount`, which acts as a public or private lobby for listing games.                                                                                                                 | Game Host / dApp Backend         |
| `PublishGame`                 | Mints a game's WASM bundle as an NFT on Solana using the Metaplex standard, creating a `GameBundle` account.                                                                                                    | Game Developer                   |
| `CreateRecipient`             | Creates a new `RecipientAccount` with one or more `slots` for handling complex, multi-party payment distributions.                                                                                              | Game Owner / dApp Backend        |
| `AddRecipientSlot`            | Adds a new asset distribution slot to an existing `RecipientAccount`.                                                                                                                                           | Recipient Owner                  |
| `AssignRecipient`             | Assigns ownership of an unassigned share within a `RecipientSlot` to a specific address.                                                                                                                        | Recipient Owner                  |
| `RecipientClaim`              | Allows a wallet to claim its share of the assets held within one or more slots of a `RecipientAccount`.                                                                                                         | Share Owner (Player, Host, etc.) |
| `AttachBonus`                 | Attaches one or more SPL token accounts as potential bonuses for a game, identified by a string identifier.                                                                                                     | Game Owner / Sponsor             |
