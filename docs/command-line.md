# Command Line(__TBD__)

Sub project `race-cli` implements a command line tool to use RACE Protocol.

## Writing data onto chains
```
race-cli <SUBCOMMAND> [<CHAIN>] [<PATH>]

```
### Publish Game Bundle

```
race-cli publish <NAME> <CHAIN> <BUNDLE>

```
**NOTE**
1. `<NAME>` is your game's name, in one word
2. `<BUNDLE>` is the URL pointing to the `metadata.json` that already has been
uploaded to a decentralized storage IPFS such as [Arweave](https://arweave.app).

Running this command will mint an NFT representing your game and
return its address.  The caller will receive the NFT in its wallet.
It is recommenced to view the result on [Solscan](https://solscan.io/) or
[Solana Explorer](https://explorer.solana.com/).

### Create Game Account
```
race-cli create-game /path/to/game.spec.json
```

`game.spec.json` file should contain the below information:

``` json
{
  "title": "<GAME TITLE>",
  "reg_addr": "<REPLACE WITH THE REGISTRATION ADDRESS>",
  "bundle_addr": "<REPLACE WITH THE BUNDLE ADDRESS>",
  "token_addr": "<REPLACE WITH THE TOKEN ADDRESS>",
  "max_players": <number>,
  "min_deposit": <number>,
  "max_deposit": <numer>,
  "data": []
}
```
**NOTE**:
1. `<GAME TITLE>` should *not* exceed 16 characters
2. `max_players` should *not* exceed 10
3. TODO: data length

### Create Registry Account

```
race-cli create-reg
```
The registry/registration address will be returned.

### Create Player Profile
TODO


## Querying/Fetching information from chains
```
race-cli <SUBCOMMAND> <CHAIN> <ADDRESS>
```
### Query Game Account
```
race-cli game-info <CHAIN> <ADDRESS>
```

This command queries the on-chain data of a game account.

### Query Server Account

```
race-cli server-info <CHAIN> <ADDRESS>
```

This command queries the on-chain data of a server account.

### Query Registartion Account

```
race-cli reg-info <CHAIN> <ADDRESS>

```
This command queries the on-chain data of a registration account.

### Query Game Bundle

```
race-cli bundle-info <CHAIN> <ADDRESS>
```

This command queries the on-chain data of a game bundle account.
