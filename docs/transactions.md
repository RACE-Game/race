# Transactions

The primary goal of Race is to build a trustworthy web3 game infrastructure, thus we use blockchain as the source of the trust.  A general contract works together with general servers to provide the fundamentals for game development.  This article will enumerate all important transaction instructions in Race.

## Join game

A player client can join the game, by sending a transaction to contract, with his token assets and encryption public keys.  The contract will save its information in game account, which is public to everyone, so that everyone in the game would know a new player came, and use its public key to encrypt the private messages.  The transactor server will pick up new player into the game when it sees the update on chain.

![join game](join.jpg)

## Settle and Exit Game

The player client doesn't leave the game by sending transaction directly, instead the leaving is handled in settle transaction sent by the Transactor.  Whenever a game is completed, a settle transaction is prepared.  Depends on the performance of target blockchain, multiple instructions may be compressed into one to reduce the tansaction time.  A settle transaction indicates how assets are transferred amoung players, and pay out the assets for leaving players.  The contract only accepts settle transactions sent by Transactor, unless others vote for its inactive.

![exit game](exit.jpg)

## Serve

A server can serve a server by talking to the contract to write itself into game account.  The first account will be considered as Transactor when the game starts.  To serve a game, the server has to stake something, this mechanism can help to get rid of abusing.

## Game Bundle

Game bundle is an address refers to a WASM package which satisfies the Race protocol.  Bundles can be either served by general servers or executed by clients.  Since every nodes use the same bundle for same game and all deal with the same event stream(either by encrypted or not), calculation results will be the same for all.  The game publish is done by talking to the contract, an account will be created for the bundle, data will be stored in decentralized storage, e.g. IPFS or Arweave.

## Create Game Account

A game account can be created with some properties and an address that refers to a game bundle.  Each game account has some slots for players and some for servers as well. There's usually a minor cost for creating game accounts, but be the owner of the game, you will have the game commisions as benefits.

## Vote

Vote transactions can be sent by everyone in the game, including the clients and the servers.  The vote is usually used to indicate that Transactor is not working.  Once the contract received enough votes, it will halt the game, and waiting for governance interventions.
