#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

echo "Delete test db files"
rm -rf dev/test.*.db
echo "Start facade server"
just dev-facade -g <(make_cash "NLHE" "FACADE_USDC" 6 1000000 2000000 0 "holdem") \
     -g <(make_cash "PLO 6 Max" "FACADE_USDC" 6 1000000 2000000 0 "omaha") \
     -b ../race-holdem/target/race_holdem_cash.wasm
