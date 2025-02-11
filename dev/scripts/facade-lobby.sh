#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

echo "Delete test db files"
rm -rf dev/test.*.db

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

echo "Start facade server"

just dev-facade -g <(make_tourney "A completed one" "FACADE_USDC" ${START_TIMES[0]}) \
     -g <(make_tourney "Another completed" "FACADE_NATIVE" ${START_TIMES[1]}) \
     -g <(make_tourney "Upcoming" "FACADE_USDC" ${START_TIMES[2]}) \
     -g <(make_tourney "Native token" "FACADE_NATIVE" ${START_TIMES[3]}) \
     -g <(make_tourney "See ya tomorrow" "FACADE_USDC" ${START_TIMES[4]}) \
     -g <(make_tourney "Too far to start" "FACADE_USDC" ${START_TIMES[5]}) \
     -g <(make_cash "USDC 1" "FACADE_USDC") \
     -g <(make_cash "USDC 2" "FACADE_USDC") \
     -g <(make_cash "USDC 3" "FACADE_USDC") \
     -g <(make_cash "USDC 4" "FACADE_USDC") \
     -g <(make_cash "USDC 5" "FACADE_USDC") \
     -g <(make_cash "USDC 6" "FACADE_USDC") \
     -g <(make_cash "NATIVE 1" "FACADE_NATIVE") \
     -g <(make_cash "NATIVE 2" "FACADE_NATIVE") \
     -g <(make_cash "NATIVE 3" "FACADE_NATIVE") \
     -g <(make_cash "NATIVE 4" "FACADE_NATIVE") \
     -g <(make_cash "NATIVE 5" "FACADE_NATIVE") \
     -g <(make_cash "NATIVE 6" "FACADE_NATIVE") \
     -g <(make_ltmtt "Super Week" "FACADE_USDC" ${START_TIMES[2]}) \
     -b ../race-holdem/target/race_holdem_mtt_table.wasm \
     -b ../race-holdem/target/race_holdem_cash.wasm
