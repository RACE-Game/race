#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
. $ROOT/dev/scripts/misc.sh

echo "Delete test db files"
rm -rf dev/test.*.db
echo "Start facade server"
just dev-facade -g <(make_cash "3 Max" "FACADE_USDC" 3) \
     -g <(make_cash "6 Max" "FACADE_USDC" 6) \
     -g <(make_cash "8 Max" "FACADE_USDC" 8) \
     -b ../race-holdem/target/race_holdem_cash.wasm
