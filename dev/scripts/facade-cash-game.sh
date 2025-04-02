#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

echo "Delete test db files"
rm -rf dev/test.*.db
echo "Start facade server"
just dev-facade -g <(make_cash "3 Max ANTE" "FACADE_USDC" 3 1000000 2000000 2000000) \
     -g <(make_cash "6 Max" "FACADE_USDC" 6 1000000 2000000 0) \
     -g <(make_cash "8 Max" "FACADE_USDC" 8 1000000 2000000 0) \
     -b ../race-holdem/target/race_holdem_cash.wasm
