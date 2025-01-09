TIME=$(date +%s%3N)
START_TIME=$(expr $TIME + 60000)
CLOSE_TIME=$(expr $TIME + 120000)
SETTLE_TIME=$(expr $TIME + 1800000)
TICKET=100000000
TABLE_SIZE=9
START_CHIPS=100000

echo "Delete test db files"
rm dev/test.*.db

echo "Current timestamp is $START_TIME"

echo "Ticket is $TICKET"
echo "Table size is $TABLE_SIZE"
echo "Start chips is $START_CHIPS"

DATA=$(cd ./js/borsh; npx ts-node ./bin/cli.ts \
			  -u64 "$START_TIME" \
			  -u64 "$CLOSE_TIME" \
			  -u64 "$SETTLE_TIME" \
			  -u8 "$TABLE_SIZE" \
			  -u32 0 \
			  -u64 10000 \
			  -s "raceholdemtargetraceholdemmtttablewasm")
echo "DATA is $DATA"

JSON=$(cat <<EOF
{
  "title": "LTMTT",
  "bundle": "../race-holdem/target/race_holdem_ltmtt.wasm",
  "token": "FACADE_USDC",
  "maxPlayers": 10,
  "entryType": {
    "cash": {
      "minDeposit": 10,
      "maxDeposit": 300000000
    }
  },
  "data": $DATA
}
EOF
    )

echo "$JSON"
echo "$JSON" > /tmp/race-mtt-facade.json
echo "Start facade server"
just dev-facade -g /tmp/race-mtt-facade.json -b ../race-holdem/target/race_holdem_mtt_table.wasm
