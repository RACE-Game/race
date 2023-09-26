Race Protocol: A multi-chain infrastructure for asymmetric competitive games

# Master(Unreleased)

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
