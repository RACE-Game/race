# Command Line(__TBD__)

Sub project `race-cli` implements a command line tool for using the protocol.

## Query Game Account

```
Usage: race-cli game-info <CHAIN> <ADDRESS>

```
This command queries the on-chain data of a game account.

## Query Server Account

> Usage: race-cli server-info <CHAIN> <ADDRESS>

This command queries the on-chain data of a server account.

## Query Registartion Account

> Usage: race-cli reg-info <CHAIN> <ADDRESS>

This command queries the on-chain data of a registration account.

## Query Game Bundle

> Usage: race-cli bundle-info <CHAIN> <ADDRESS>

This command queries the on-chain data of a game bundle account.

## Publish Game Bundle

> Usage: race-cli publish <CHAIN> <BUNDLE>

The wasm file will be uploaded to a decentralized storage,
IPFS/Arweave.  Then a NFT will be created, refers to that link.  The
caller will receive the NFT in its wallet.
