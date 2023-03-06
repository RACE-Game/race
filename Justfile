set dotenv-load

build: sdk transactor cli

dep:
    cargo fetch
    npm --prefix ./examples/demo-app i

sdk:
    wasm-pack build --release --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

dev-sdk:
    wasm-pack build --dev --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

facade:
    cargo build -r -p race-facade

transactor:
    cargo build -r -p race-transactor

cli:
    cargo build -r -p race-cli


test: test-transactor

test-transactor:
    cargo test -p race-transactor


examples: example-minimal example-counter example-chat example-raffle

example-minimal:
    cargo build -r -p race-example-minimal --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_minimal.wasm -o target/race_example_minimal.wasm

example-counter:
    cargo build -r -p race-example-counter --target wasm32-unknown-unknown

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

dev-transactor-1:
    cargo run -p race-transactor -- -c ./examples/conf/race_server_1.toml run

dev-transactor-2:
    cargo run -p race-transactor -- -c ./examples/conf/race_server_2.toml run

alias fa := dev-facade
alias t1 := dev-transactor-1
alias t2 := dev-transactor-2
alias da := dev-demo-app
