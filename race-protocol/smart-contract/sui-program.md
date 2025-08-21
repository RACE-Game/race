---
description: Sui Move Package
icon: sun
---

# Sui Program

The Sui implementation of Race Protocol is structured as a series of Move modules within the `race_sui` package. Unlike Solana's single-program model, Sui transactions are composed by chaining together calls to these public functions in a Programmable Transaction Block (PTB).

The primary modules and their public entry functions are listed below. These functions serve as the building blocks for all on-chain interactions.

### Module Reference

| Function                      | Module      | Description                                                                                                                                                    |
| ----------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Game Management**           |             |                                                                                                                                                                |
| `create_game`                 | `game`      | Creates and shares a new on-chain `Game` object, analogous to Solana's `CreateGameAccount`.                                                                    |
| `close_game`                  | `game`      | Deletes an empty `Game` object and its associated assets, reclaiming storage gas. Can only be called by the game owner.                                        |
| **Player Management**         |             |                                                                                                                                                                |
| `create_profile`              | `profile`   | Creates a `PlayerProfile` object and transfers it to the sender.                                                                                               |
| `update_profile`              | `profile`   | Updates the `nick` and `pfp` of an existing `PlayerProfile` object.                                                                                            |
| `join_game`                   | `game`      | Allows a player to join a game by providing the required payment coins.                                                                                        |
| `deposit`                     | `game`      | Allows an existing player to deposit additional funds into a game.                                                                                             |
| **Server Management**         |             |                                                                                                                                                                |
| `register_server`             | `server`    | Creates a `Server` object and transfers it to the server operator's address.                                                                                   |
| `serve_game`                  | `game`      | Allows a registered server to join a game's `servers` list. The first server becomes the transactor.                                                           |
| **Asset & Bundle Management** |             |                                                                                                                                                                |
| `create_registry`             | `registry`  | Creates a new `Registry` object for listing games, which can be public or private.                                                                             |
| `register_game`               | `registry`  | Adds a Game to a `Registry` object.                                                                                                                            |
| `unregister_game`             | `registry`  | Removes a Game from a `Registry` object.                                                                                                                       |
| `publish`                     | `game`      | Mints a `GameNFT` object that contains the metadata and URI for a game bundle.                                                                                 |
| `create_recipient`            | `recipient` | Creates a `Recipient` object and its associated `RecipientSlot` objects for managing complex payments. This is a multi-step process orchestrated within a PTB. |
| `attach_bonus`                | `game`      | Attaches a `Prize` object (which can be a Coin or another NFT) to a `Game` as a potential bonus.                                                               |
| `recipient_claim`             | `recipient` | Allows a user to claim their share of funds from a `RecipientSlot`.                                                                                            |

### Settlement on Sui

Unlike Solana, where settlement is a single, complex instruction, settlement on Sui is performed via a **Programmable Transaction Block (PTB)**. The Transactor constructs a PTB that calls a sequence of `public(package)` functions from the settle module. This provides greater flexibility and helps manage transaction complexity.

A typical settlement PTB would involve calls to:

1. **`pre_settle_checks`**: A mandatory first step that validates the sender's authority and the game's versioning, returning a `CheckPass` object that is required by subsequent settlement functions.
2. **`handle_settles`**: Processes all player balance changes, withdrawals, and ejections.
3. **`handle_transfer`**: Manages the transfer of funds (e.g., rake) to the game's `Recipient` account.
4. **`handle_bonus`**: Distributes any awarded bonuses (NFTs or tokens) to winning players.
5. **`finish_settle`**: The final step that updates the game's `settle_version`, writes the new `checkpoint`, and manages accepted deposits.
