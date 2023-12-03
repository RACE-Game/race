TIME=$(date +%s%3N)
START_TIME=$(expr $TIME + 60000)
echo "Current timestamp is $START_TIME"

DATA=$(cd ./js/borsh; npx ts-node ./bin/cli.ts \
                  -u64 "$START_TIME" \
                  -u8 2 \
                  -u64 10 \
                  -u64 60000 \
                  -u32 0)
echo "DATA is $DATA"

JSON=$(cat <<EOF
{
  "title": "MTT",
  "bundle": "../race-holdem/target/race_holdem_mtt.wasm",
  "token": "FACADE_USDC",
  "maxPlayers": 10,
  "entryType": {
    "cash": {
      "minDeposit": 5000,
      "maxDeposit": 5000
    }
  },
  "data": $DATA
}
EOF
    )

echo "$JSON"
echo "$JSON" > /tmp/race-mtt-facade.json
just dev-facade /tmp/race-mtt-facade.json
