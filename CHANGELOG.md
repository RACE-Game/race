Race Protocol: A multi-chain infrastructure for asymmetric competitive games

# Master(Unreleased)

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
