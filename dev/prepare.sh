#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

set -e

echo "Preparing the local environment..."
echo "The keypair at $HOME/.config/solana/id.json will be used"
echo "Create a SPL token for testing..."


if solana config get | grep 'RPC URL: http://localhost:8899';
then
    echo "Confirmed LOCAL network is used"
else
    echo "Exit due to mainnet is used"
    exit 1
fi

echo "Airdrop 10SOL..."
solana airdrop 10

TOKEN=$(spl-token create-token  --decimals 0 | grep 'Address' | awk '{print $2}')
echo "Token: $TOKEN will be used for games"
spl-token create-account "$TOKEN"
spl-token mint $TOKEN 1000000

mkdir -p dist
echo "Generate game bundle NFT metadata..."
cat <<EOF > dist/race_example_raffle_metadata.json
{
 "name": "RACE Example Raffle",
 "symbol": "RACE",
 "description": "Example raffle game",
 "seller_fee_basis_points": 0,
 "image": "https://arweave.net/OQx85OOqT0oawKjCS9f5PowBF_5Y8hkJ8DI5fvMjd-s",
 "external_url": "",
 "attributes": [],
 "properties": {
   "files": [
     {
       "uri": "https://arweave.net/OQx85OOqT0oawKjCS9f5PowBF_5Y8hkJ8DI5fvMjd-s",
       "type": "image/png"
     },
     {
       "uri": "http://127.0.0.1:8000/race_example_raffle.wasm",
       "type": "application/wasm"
     }
   ],
   "category": "image",
   "creators": [
     {
       "address": "J22ir2nLxVRqUrcpwMDBM47HpFCLLRrKFroF6LjK7DEA",
       "share": 100
     }
   ]
 }
}
EOF

echo "Compile Raffle example..."
just example-raffle
cp ../target/race_example_raffle.wasm  dist/

echo "Publishing WASM as game bundle NFT"
BUNDLE=$(just publish Raffle 'http://127.0.0.1:8000/race_example_raffle_metadata.json' | grep 'Address' | awk '{print $2}')
echo "Raffle is published as NFT at $BUNDLE"

echo "Create registration center..."
REG=$(just create-reg | grep 'Address' | awk '{print $2}')
echo "Registration: $REG will be used"

echo "Generate game spec..."
cat <<EOF > dist/raffle.spec.json
{
  "title": "My Raffle",
  "reg_addr": "$REG",
  "bundle_addr": "$BUNDLE",
  "token_addr": "$TOKEN",
  "max_players": 10,
  "min_deposit": 1,
  "max_deposit": 2,
  "data": []
}
EOF

echo "Create game..."
GAME=$(just create-game $SCRIPT_DIR/dist/raffle.spec.json | grep 'Address' | awk '{print $2}')
echo "A game of raffle is created at $GAME"

echo "Generate a keypair for transactor..."
solana-keygen new -o dist/transactor-keypair.json --no-bip39-passphrase -s -f
# solana airdrop 10 dist/transactor-keypair.json
TRANSACTOR=$(solana-keygen pubkey dist/transactor-keypair.json)
solana airdrop 10 dist/transactor-keypair.json --commitment finalized

echo "Generate server configuration..."
TX_PATH=$SCRIPT_DIR/dist/transactor-keypair.json
cat <<EOF > dist/transactor.toml
[transactor]
port = 12003
endpoint = "ws://localhost:12003"
chain = "solana"
address = "$TRANSACTOR"
reg_addresses = ["$REG"]

[solana]
keyfile = "$TX_PATH"
rpc = "http://localhost:8899"
EOF

echo "Create server account..."
just dev-reg-transactor $SCRIPT_DIR/dist/transactor.toml

echo "Generate demo-app-data..."
cat <<EOF > dist/demo-app-data.json
{
  "CHAIN_TO_REG_ADDR": {
    "solana-local": "$REG"
  },
  "CHAIN_ADDR_GAME_MAPPING": {
    "solana-local": {
       "$BUNDLE": "raffle"
    }
  }
}
EOF

echo "That's it!
Start a server to simulate AR storage and provide data for demo-app: simple-http-server --cors -- dist
Now you can start the transactor with: just dev-transactor ${SCRIPT_DIR}/dist/transactor.toml
And open the demo app with: just ts-sdk dev-sdk dev-demo-app
"
