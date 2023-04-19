set dotenv-load

build: sdk transactor cli

dev-sdk:
    wasm-pack build --debug --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

sdk:
    wasm-pack build --release --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

facade:
    cargo build -r -p race-facade

transactor:
    cargo build -r -p race-transactor

cli:
    cargo build -r -p race-cli


test: test-core test-transactor

test-transactor:
    cargo test -p race-transactor

test-core:
    cargo test -p race-core

examples: example-chat example-raffle preview-demo-app

example-chat:
    cargo build -r -p race-example-chat --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_chat.wasm -o target/race_example_chat.wasm

example-raffle:
    cargo build -r -p race-example-raffle --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_raffle.wasm -o target/race_example_raffle.wasm

dev-demo-app:
    npm --prefix ./examples/demo-app run dev

demo-app:
    npm --prefix ./examples/demo-app run build

preview-demo-app: demo-app
    npm --prefix ./examples/demo-app run preview

dev-facade:
    cargo run -p race-facade

dev-transactor conf:
    cargo run -p race-transactor -- -c {{conf}} reg
    cargo run -p race-transactor -- -c {{conf}} run

solana:
    (cd contracts/solana; cargo build-sbf)
    solana program deploy ./target/deploy/race_solana.so

ts-sdk:
    npm --prefix ./js/sdk-core run build:js
    npm --prefix ./js/sdk-core run build:typedefs
    npm --prefix ./js/sdk-solana run build:js
    npm --prefix ./js/sdk-solana run build:typedefs

publish name url:
    cargo run -p race-cli -- -e local publish solana {{name}} {{url}}

create-reg:
    cargo run -p race-cli -- -e local create-reg solana

create-game spec:
    cargo run -p race-cli -- -e local create-game solana {{spec}}

alias fa := dev-facade
alias da := dev-demo-app
