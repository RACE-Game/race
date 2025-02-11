echo "Delete test db files"
rm -rf dev/test.*.db
echo "Start facade server"
just dev-facade -g dev/game-specs/holdem-cash.json -g dev/game-specs/holdem-cash-2.json
