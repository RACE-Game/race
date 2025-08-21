---
description: Command-Line Interface
---

# 💻 RACE CLI

The `race-cli` is a powerful command-line tool for developers and node operators to interact with the RACE Protocol. It allows you to manage on-chain assets like game accounts, game bundles, and registration centers directly from your terminal.

***

## **Usage**

The CLI is structured with global options followed by a subcommand and its specific arguments.

**Syntax:**

```sh
# Using the Just command from the project root
just cli -- [GLOBAL_OPTIONS] <SUBCOMMAND> [ARGS]

# Or running directly with Cargo
cargo run -p race-cli -- [GLOBAL_OPTIONS] <SUBCOMMAND> [ARGS]
```

**Global Options**

These options must be provided before the subcommand.

* `-c`, `--chain <CHAIN>`: **(Required)** The blockchain to interact with. Valid options are solana, sui, and facade.
* `-r`, `--rpc <RPC>`: **(Required)** The RPC endpoint for the specified chain. For convenience, you can use shortcuts for Solana networks: `mainnet` (or `m`), `testnet` (or `t`), `devnet` (or `d`), or `local` (or `l`).
* `-k`, `--keyfile <KEYFILE>`: (Optional) The file path to your wallet keypair. If you use "default", the CLI will use the default keyfile location for the specified chain (e.g., `~/.config/solana/id.json` for Solana).
* `-a`, `--arweave-keyfile <ARWEAVE_KEYFILE>`: (Optional) The path to your Arweave JWK keyfile. This is required for commands that upload data to Arweave, such as `publish`.

***

## **Game Bundle Management**

Commands for publishing and querying game bundles (WASM).

### **`publish`**

Uploads a game bundle to Arweave, creates its on-chain metadata, and mints it as an NFT.

* **Description**: Publish a game bundle.
* **Usage**: `race-cli ... publish <NAME> <SYMBOL> <CREATOR> <BUNDLE>`
* **Arguments**:
  * `<NAME>`: The name of the game (e.g., "My Awesome Game").
  * `<SYMBOL>`: The token symbol for the game's NFT (e.g., "MYGAME").
  * `<CREATOR>`: The address of the creator/developer.
  * `<BUNDLE>`: The local file path to the game's compiled WASM bundle.

### **`download-bundle`**

Downloads a game bundle's WASM file from its on-chain address.

* **Description**: Download a game bundle.
* **Usage**: `race-cli ... download-bundle <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The on-chain address of the game bundle.

### **`mint-nft`**

Mints a game bundle NFT using a pre-existing Arweave URL.

* **Description**: Mint NFT with an Arweave URL.
* **Usage**: `race-cli ... mint-nft <NAME> <SYMBOL> <ARWEAVE_URL>`
* **Arguments**:
  * `<NAME>`: The name of the game.
  * `<SYMBOL>`: The token symbol for the game's NFT.
  * `<ARWEAVE_URL>`: The Arweave URL pointing to the game's metadata.

### **`bundle-info`**

Queries and displays information about a game bundle.

* **Description**: Query game bundle information.
* **Usage**: `race-cli ... bundle-info <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The on-chain address of the game bundle.

***

## **Account Information**

Commands for querying the state of various on-chain accounts.

### **`game-info`**

Displays detailed information about a specific game account.

* **Description**: Query game account information.
* **Usage**: `race-cli ... game-info <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The address of the game account.
* **Output**: Shows title, owner, players, servers, deposits, checkpoint status, balances, and a hex dump of the game's custom data.

### **`server-info`**

Displays information about a registered server account.

* **Description**: Query server account information.
* **Usage**: `race-cli ... server-info <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The address of the server account.

### **`reg-info`**

Displays information about a registration center, including a list of registered games.

* **Description**: Query registration center.
* **Usage**: `race-cli ... reg-info <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The address of the registration account.

### **`recipient-info`**

Displays detailed information about a recipient account, including its slots and share distribution.

* **Description**: Query recipient account.
* **Usage**: `race-cli ... recipient-info <ADDRESS>`
* **Arguments**:
  * `<ADDRESS>`: The address of the recipient account.

***

## **Account Creation & Management**

Commands for creating and managing on-chain accounts.

### **`create-reg`**

Creates a new public registration center for listing games.

* **Description**: Create registration center.
* **Usage**: `race-cli ... create-reg`

## **`create-game`**

Creates a new game account from a JSON specification file.

* **Description**: Create game account.
* **Usage**: `race-cli ... create-game <SPEC_FILE>`
* **Arguments**:
  * `<SPEC_FILE>`: Path to a JSON file specifying the game's properties.
*   **Specification File (`<SPEC_FILE>`):**

    ```json
    {
      "title": "My Texas Hold'em Game",
      "regAddr": "...",
      "tokenAddr": "...",
      "bundleAddr": "...",
      "maxPlayers": 6,
      "entryType": {
        "cash": {
          "minDeposit": 1000000,
          "maxDeposit": 10000000
        }
      },
      "recipient": {
        "addr": "..."
      },
      "data": [ ... ]
    }
    ```

    * **recipient**: This field can take one of two forms:
      * An existing recipient address: `{ "addr": "..." }`
      * A definition to create a new recipient account: `{ "slots": [ ... ] }` (See create-recipient for the slot structure).
    * **data**: An array of u8 bytes representing the borsh-serialized initial game data.

### **`create-recipient`**

Creates a new recipient account for managing asset distribution.

* **Description**: Create recipient account.
* **Usage**: `race-cli ... create-recipient <SPEC_FILE>`
* **Arguments**:
  * `<SPEC_FILE>`: Path to a JSON file specifying the recipient's structure.
*   **Specification File (`<SPEC_FILE>`):**

    ```json
    {
      "slots": [
        {
          "id": 0,
          "slotType": "Token",
          "tokenAddr": "...",
          "initShares": [
            {
              "owner": { "unassigned": { "identifier": "house_commission" } },
              "weights": 1000
            },
            {
              "owner": { "assigned": { "addr": "..." } },
              "weights": 9000
            }
          ]
        }
      ]
    }
    ```

### **`reg-game`**

Registers an existing game account with a registration center.

* **Description**: Register game account.
* **Usage**: `race-cli ... reg-game <REG_ADDRESS> <GAME_ADDRESS>`

### **`unreg-game`**

Unregisters a game account from a registration center.

* **Description**: Unregister game account.
* **Usage**: `race-cli ... unreg-game <REG_ADDRESS> <GAME_ADDRESS> [--close]`
* **Options**:
  * `--close`: If specified, the game account will be closed after being unregistered.

### **`close-game`**

Closes an empty game account to reclaim rent.

* **Description**: Close game account.
* **Usage**: `race-cli ... close-game <GAME_ADDRESS>`

### **`close-all-games`**

Unregisters and closes all games associated with a specific registration center.

* **Description**: Unregister and close all games for a registration.
* **Usage**: `race-cli ... close-all-games <REG_ADDRESS>`

### **`add-recipient-slot`**

Adds a new slot to an existing recipient account.

* **Description**: Add slot to a recipient.
* **Usage**: `race-cli ... add-recipient-slot <RECIPIENT_ADDRESS> <SPEC_FILE>`
* **Arguments**:
  * `<RECIPIENT_ADDRESS>`: The address of the recipient account.
  * `<SPEC_FILE>`: A JSON file containing the slot(s) to add, with the structure {"slots": \[ ...]}.

***

## **Payment Commands**

### **`claim`**

Claims payable tokens from a recipient account for the provided keypair.

* **Description**: Claim tokens from a recipient account.
* **Usage**: `race-cli ... claim <RECIPIENT_ADDRESS>`
* **Arguments**:
  * `<RECIPIENT_ADDRESS>`: The address of the recipient account to claim from.
