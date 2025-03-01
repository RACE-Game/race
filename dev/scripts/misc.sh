# Make a JSON specification for tourney game
# TITLE TOKEN START_TIME
function make_tourney {
    local ENTRY_CLOSE_TIME=$(expr $3 + 300000)
    local TICKET=100000000
    local TABLE_SIZE=2
    local START_CHIPS=100000

    local data=$(cd $ROOT/js/borsh; npx ts-node ./bin/cli.ts \
                             -u64 $3 \
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
  "token": "$2",
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

# Make a JSON specification for cash game
# TITLE TOKEN
function make_cash {
    local json=$(cat <<EOF
{
    "title": "$1",
    "bundle": "../race-holdem/target/race_holdem_cash.wasm",
    "token": "$2",
    "maxPlayers": 6,
    "entryType": {
        "cash": {
            "minDeposit": 100000000,
            "maxDeposit": 300000000
        }
    },
    "data": [64,66,15,0,0,0,0,0,128,132,30,0,0,0,0,0,0,0,0,0,0,0,0,0,30,0,1,0,163,225,17,0,0,0,0,0]
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
