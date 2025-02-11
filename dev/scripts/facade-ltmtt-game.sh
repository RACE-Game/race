#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

TIME=$(date +%s%3N)
START_TIME=$(expr $TIME + 60000)

echo "Delete test db files"
rm -rf dev/test.*.db

echo "Current timestamp is $START_TIME"

echo "Ticket is $TICKET"
echo "Table size is $TABLE_SIZE"
echo "Start chips is $START_CHIPS"

echo "Start facade server"
just dev-facade -g <(make_ltmtt "Super Week" "FACADE_USDC" $START_TIME) \
     -b ../race-holdem/target/race_holdem_mtt_table.wasm
