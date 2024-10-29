TIME=$(date +%s%3N)
START_TIME=$(expr $TIME + 30000)
TICKET=100000000
TABLE_SIZE=3
START_CHIPS=10000

echo "Delete test db"
rm dev/test.db

echo "Current timestamp is $START_TIME"
echo "Ticket is $TICKET"
echo "Table size is $TABLE_SIZE"
echo "Start chips is $START_CHIPS"


DATA=$(cd ./js/borsh; npx ts-node ./bin/cli.ts \
                          -u64 "$START_TIME" \
                          -u64 "$TICKET" \
                          -u8 "$TABLE_SIZE" \
                          -u64 "$START_CHIPS" \
                          -u64 50 \
                          -u64 60000 \
                          -u32 0 \
                          -u32 3 \
                          -u8 50 \
                          -u8 30 \
                          -u8 20 \
                          -u8 0)
echo "DATA is $DATA"

JSON=$(cat <<EOF
{
  "title": "MTT",
  "bundle": "../race-holdem/target/race_holdem_mtt.wasm",
  "token": "FACADE_USDC",
  "maxPlayers": 10,
  "entryType": {
    "cash": {
      "minDeposit": 100000000,
      "maxDeposit": 100000000
    }
  },
  "data": $DATA
}
EOF
    )

echo "$JSON"
echo "$JSON" > /tmp/race-mtt-facade.json
just dev-facade -g /tmp/race-mtt-facade.json -b ../race-holdem/target/race_holdem_mtt_table.wasm
