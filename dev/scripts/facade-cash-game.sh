echo "Delete test db files"
rm dev/test.*.db
echo "Start facade server"
just dev-facade -g dev/game-specs/holdem-cash.json