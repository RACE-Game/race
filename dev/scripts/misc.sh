# Make a JSON specification for tourney game
# TITLE TOKEN START_TIME
function make_tourney {
    local ENTRY_CLOSE_TIME=$(expr $3 + 300000)
    local TICKET=100000000
    local TABLE_SIZE=4
    local START_CHIPS=100000

    local data=$(cd $ROOT/js/borsh; npx ts-node ./bin/cli.ts \
                             -u64 $3 \
                             -u64 $ENTRY_CLOSE_TIME \
                             -u64 $TICKET \
                             -u8 $TABLE_SIZE \
                             -u64 $START_CHIPS \
                             -u64 2000 \
                             -u64 60000 \
                             -u32 0 \
                             -u32 2 \
                             -u8 70 \
                             -u8 30 \
                             -u16 2 \
                             -u16 50 \
                             -u16 10 \
                             -u8 0 \
                             -s "raceholdemtargetraceholdemmtttablewasm")

    local json=$(cat <<EOF
{
  "title": "$1",
  "bundle": "../race-holdem/target/race_holdem_mtt.wasm",
  "token": "$2",
  "maxPlayers": 100,
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

# Make a JSON specification for cash game
# TITLE TOKEN
function make_cash {

    local MIN_DEPOSIT=$(($5 * 50))
    local MAX_DEPOSIT=$(($5 * 100))

    local data=$(cd $ROOT/js/borsh; npx ts-node ./bin/cli.ts \
                                        -u64 $4 \
                                        -u64 $5 \
                                        -u64 $6 \
                                        -u16 10 \
                                        -u8 1 \
                                        -u64 $MAX_DEPOSIT \
                                        -u8 0)

    local json=$(cat <<EOF
{
    "title": "$1",
    "bundle": "../race-holdem/target/race_holdem_cash.wasm",
    "token": "$2",
    "maxPlayers": $3,
    "entryType": {
        "cash": {
            "minDeposit": $MIN_DEPOSIT,
            "maxDeposit": $MAX_DEPOSIT
        }
    },
    "data": $data
}

EOF
          )

    echo $json
}

# Make a JSON specification for long-term tournament
# TITLE TOKEN START_TIME
function make_ltmtt {
    local CLOSE_TIME=$(expr $3 + 600000)
    local SETTLE_TIME=$(expr $3 + 720000)
    local TABLE_SIZE=8

    local data=$(cd ./js/borsh; npx ts-node ./bin/cli.ts \
			  -u64 "$3" \
			  -u64 "$CLOSE_TIME" \
			  -u64 "$SETTLE_TIME" \
			  -u8 "$TABLE_SIZE" \
			  -u32 0 \
			  -u16 10 \
			  -u32 3 \
			  -u8 50 \
			  -u8 30 \
			  -u8 20 \
			  -s "raceholdemtargetraceholdemmtttablewasm")

    local json=$(cat <<EOF
{
  "title": "$1",
  "bundle": "../race-holdem/target/race_holdem_ltmtt.wasm",
  "token": "$2",
  "maxPlayers": 10,
  "entryType": {
    "cash": {
      "minDeposit": 0,
      "maxDeposit": 300000000
    }
  },
  "data": $data
}
EOF
          )

    echo $json
}
