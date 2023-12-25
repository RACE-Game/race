Race Protocol: A multi-chain infrastructure for asymmetric competitive games

# Master(Unreleased)

## Breaking Changes
- Facade: Argument format updated. Now it receives `-g` for game, and `-b` for bundle.

## Enhancements
- SDK: Remove dependency `crypto` to support NodeJS runtime.
- CLI: Print `checkpoint` in hex format in command `game-info`.
- TestKit: Add `handle_dispatch_until_no_events` to TestHandler.

## Features
- CLI: Update `publish` command. Now it receives the path to the WASM bundle instead of the Arweave URL to solana metadata.
- Add optional `createProfileIfNeeded` to join options.

## Fixes
- Transactor: Improve the retry mechanism for settle.
- SDK: Load the legacy solana token list from arweave.

# 0.2.6

## Fixes
- SDK: Clean the timer after exit. Simplify the usage of timer in connection.
- Transactor: Log serialized effect and event when a wasm runtime error occurs.
- SDK: Fix broken on consecutive `SecretsReady` events.

## Enhancements
- SDK: Add fake NFT data to facade transport.

# 0.2.5

## Fixes
- Remove duplicated definitions for `SecretIdent` and `SecretShare`, which causes the compliation error.

# 0.2.4

## Features
- SDK: Add `npx borsh-serialize` to js/borsh.

## Enhancements
- Transactor & Contract: Squash transactions for better performance.
- CLI: Display token address in `game-info`.

## Fixes
- Transactor: Fix sending duplicated settlements.
- SDK: Fix double reconnecting.

# 0.2.3

## Enhancements
- CLI: Recover fees in `unreg-game`.
- CLI: Display slot's balance in `recipient-info` command.

## Fixes
- Solana: Fix NFT caching

# 0.2.2

## Breaking changes
- Now games must implement `into_checkpoint`.

## Enhancements
- SDK & Transactor & Contract: Add checkpoint to on-chain game account so that we can reproduce correct game state without querying transactor.
- SDK: Add meaningful return value for transport API.

## Fixes
- Solana: Fix compute budget limit.

# 0.2.1

## Features
- CLI: Add `claim` command to claim tokens from a recipient account.

## Enhancements
- SDK: Add caching for token/NFT fetching.
- SDK: Add caching for game bundle fetching. (Caching for wasm module is TBD)
- SDK: Add `data` and `dataLen` to `AppHelper.info`.
- SDK: Add original metadata to NFT structure.

## Breaking changes
- CONTRACT: Remove `claim_amount_cap`.

# 0.2.0

## Breaking changes
- A minimal crate `api` is separated from `core`.  This crate will replace the old `core` and be used in game bundle.

## Features

- SDK: Add new parameter `onConnectionState` to `AppClientInitOpts`.  A function is expeceted to receive one string argument which represents the network status to transactor.
- SDK: Add reconnecting.

## Fixes
- Transactor: Add local blacklist for malformed game address.  Add `disable_blacklist = true` to `[transactor]` section in config file to disable.
- Transactor: Add task queue in transactor submitter.
- CLI: Add `recipient-info` command for querying the recipient structure by address.
- SDK: Add `bundleAddr` and `tokenAddr` to game info.
- SDK: Fix `listTokens` and `getToken` in facade transport.
- SDK: Close the websocket connection when exit game.

# 0.1.0

It basically works with a Holdem game
