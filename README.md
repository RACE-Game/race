Race Protocol is for builders, to easily create web3 asymmetric information competitive games.

# Project Status

Work in progress, not ready for contribution yet.

# Architecture Overview

## Game bundle

Games are built as side-effect free state machine for core logic,
compiled to WebAssembly, and loaded by both servers and clients.

## Server

The servers are standardized and generalized, serve game bundles.
Each game on each server can be executed in two modes: `Transactor` or
`Validator`.

With `Transactor` mode, the server sends transactions.  With
`Validator` mode, the server validates game progress and contribute
to randomization.

Game requires to be served by a cluster of independent servers that
confirm consensus reached in a decentralized way.

## Hidden Knowledge

Race Protocol provides Peer-to-Peer, Server-to-Server, Server-to-Peer
cryptographically encrypted randomization and commuinication tools
required to build single-player or multi-player hidden knowledge games and apps.

## Smart Contract

The smart contract handles all deposits and withdraws in a transparent way.
The assets must be either in user wallet or in game account.

## What about Database

No centralized database is involved, we emphasize database-free solution.

# Features

- The settlement system and corresponding contracts.
- Blockchain-based storage solution, no database required.
- RSA-based encrypted communication between server and clients.
- NFT-based game core bundle (in wasm format) publishing.
- The genereal game server that can be hosted by communties.
- P2P randomness generator,  fairness guaranteed.
- Multi-chain support.

# About the name "RACE"
> RACE = R + ACE

- *R* stands for REDEFINING
- *ACE* stands for the BEST
