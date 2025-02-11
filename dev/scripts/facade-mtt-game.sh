#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

echo "Delete test db files"
rm -rf dev/test.*.db

ROOT=$(git rev-parse --show-toplevel)
TIME=$(date +%s%3N)

echo "Ticket is $TICKET"
echo "Table size is $TABLE_SIZE"
echo "Start chips is $START_CHIPS"

START_TIMES=($(expr $TIME)
             $(expr $TIME - 600000)
             $(expr $TIME + 60000)
             $(expr $TIME + 120000)
             $(expr $TIME + 86400000)
             $(expr $TIME + 999999999))

echo "Start facade server"

just dev-facade -g <(make_tourney "Upcoming" "FACADE_USDC" ${START_TIMES[2]}) \
     -b ../race-holdem/target/race_holdem_mtt_table.wasm
