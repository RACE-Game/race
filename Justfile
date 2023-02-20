set dotenv-load

build: sdk transactor

sdk:
    cd ./sdk; wasm-pack build --release --target web
    cd ./sdk; patch pkg/package.json < package.json.patch

demo-app:
    cd ./examples/demo-app; npm run build

facade:
    cargo build -r -p race-facade

transactor:
    cargo build -r -p race-transactor


test: test-transactor

test-transactor:
    cargo test -p race-transactor


examples: example-chat example-raffle

example-chat:
    cargo build -r -p race-example-chat --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_chat.wasm -o target/race_example_chat.wasm

example-raffle:
    cargo build -r -p race-example-raffle --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_raffle.wasm -o target/race_example_raffle.wasm

dev-demo-app:
    cd ./examples/demo-app; npm run dev

dev-facade:
    cargo run -p race-facade

dev-transactor-1:
    cargo run -p race-transactor -- -c ./examples/conf/race_server_1.toml run

dev-transactor-2:
    cargo run -p race-transactor -- -c ./examples/conf/race_server_2.toml run

alias fa := dev-facade
alias t1 := dev-transactor-1
alias t2 := dev-transactor-2
alias da := dev-demo-app
