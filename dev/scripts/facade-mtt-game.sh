#!/bin/bash

set -e

echo "Delete test db files"
rm -rf dev/test.*.db

ROOT=$(git rev-parse --show-toplevel)
TIME=$(date +%s%3N)
ENTRY_CLOSE_TIME=$(expr $START_TIME + 300000)
TICKET=100000000
TABLE_SIZE=2
START_CHIPS=100000

echo "Ticket is $TICKET"
echo "Table size is $TABLE_SIZE"
echo "Start chips is $START_CHIPS"

START_TIMES=($(expr $TIME)
             $(expr $TIME - 600000)
             $(expr $TIME + 60000)
             $(expr $TIME + 120000)
             $(expr $TIME + 86400000)
             $(expr $TIME + 999999999))

function make_data_with_start_time {
    local data=$(cd $ROOT/js/borsh; npx ts-node ./bin/cli.ts \
                             -u64 $2 \
                             -u64 $ENTRY_CLOSE_TIME \
                             -u64 $TICKET \
                             -u8 $TABLE_SIZE \
                             -u64 $START_CHIPS \
                             -u64 10 \
                             -u64 60000 \
                             -u32 0 \
                             -u32 3 \
                             -u8 50 \
                             -u8 30 \
                             -u8 20 \
                             -u8 0 \
                             -s "raceholdemtargetraceholdemmtttablewasm")

    local json=$(cat <<EOF
{
  "title": "$1",
  "bundle": "../race-holdem/target/race_holdem_mtt.wasm",
  "token": "$3",
  "maxPlayers": 10,
  "entryType": {
    "ticket": {
      "slotId": 1,
      "amount": 100000000
    }
  },
  "data": $data
}
EOF
          )

    echo $json
}

echo "Start facade server"

just dev-facade -g <(make_data_with_start_time "A completed one" ${START_TIMES[0]} "FACADE_USDC") \
     -g <(make_data_with_start_time "Another completed" ${START_TIMES[1]} "FACADE_NATIVE") \
     -g <(make_data_with_start_time "Upcoming" ${START_TIMES[2]} "FACADE_USDC") \
     -g <(make_data_with_start_time "Native token" ${START_TIMES[3]} "FACADE_NATIVE") \
     -g <(make_data_with_start_time "See ya tomorrow" ${START_TIMES[4]} "FACADE_USDC") \
     -g <(make_data_with_start_time "Too far to start" ${START_TIMES[5]} "FACADE_USDC") \
     -b ../race-holdem/target/race_holdem_mtt_table.wasm
