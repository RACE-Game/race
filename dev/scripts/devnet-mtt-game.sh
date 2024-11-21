TIME=$(date +%s%3N)
START_DELAY=$(($1 * 60000))
START_TIME=$(($TIME + $START_DELAY))
TICKET=1000000
TABLE_SIZE=3
START_CHIPS=100000

DATA=$(cd ./js/borsh; npx ts-node ./bin/cli.ts \
                          -u64 "$START_TIME" \
                          -u64 "$TICKET" \
                          -u8 "$TABLE_SIZE" \
                          -u64 "$START_CHIPS" \
                          -u64 10 \
                          -u64 60000 \
                          -u32 0 \
                          -u32 3 \
                          -u8 50 \
                          -u8 30 \
                          -u8 20 \
                          -u8 0 \
                          -s "E3WPTpsDbDN8waMbgCsFpJr8HHZXjy3BBHwxUKHKdoQf")


JSON=$(cat <<EOF
{
  "title": "Test MTT",
  "regAddr": "GmmisusD5E6wLpyUAMKXrV7o8MS5fk6JtgX4xpJDBG9b",
  "tokenAddr": "2hsJHem78HuhWhk5ATAxqbanPDAtoq95Dn9M8oE7gCNA",
  "bundleAddr": "7pWmukH8EDgp8euRnnbUo8tEY8uNabKkV5HLDmaEyG2G",
  "maxPlayers": 6,
  "entryType": {
    "ticket": {
      "amount": $TICKET
    }
  },
  "recipient": {
    "addr": "AnYU7DGuq3LP37G796vZzp9pdfhk4ForPv47A6DhcVwv"
  },
  "data": $DATA
}
EOF
    )

echo $JSON
