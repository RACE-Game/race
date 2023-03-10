set dotenv-load

build: build-sdk build-transactor build-cli

dep:
    cargo fetch
    npm --prefix ./examples/demo-app i

build-sdk:
    wasm-pack build --release --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

dev-sdk:
    wasm-pack build --dev --target web sdk
    patch ./sdk/pkg/package.json < ./sdk/package.json.patch

build-facade:
    cargo build -r -p race-facade

build-transactor:
    cargo build -r -p race-transactor

build-cli:
    cargo build -r -p race-cli

cli *ARGS:
    cargo run -p race-cli -- {{ARGS}}

test: test-transactor

test-transactor:
    cargo test -p race-transactor


examples: example-minimal example-chat example-raffle example-draw-card

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

example-draw-card:
    cargo build -r -p race-example-draw-card --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_draw_card.wasm -o target/race_example_draw_card.wasm

dev-demo-app:
    npm --prefix ./examples/demo-app run dev

demo-app:
    npm --prefix ./examples/demo-app run build

preview-demo-app: demo-app
    npm --prefix ./examples/demo-app run preview

dev-facade:
    cargo run -p race-facade

dev-transactor CONF:
    cargo run -p race-transactor -- -c {{CONF}} run

alias fa := dev-facade
t1:
    cargo run -p race-transactor -- -c ./examples/conf/server1.toml run
t2:
    cargo run -p race-transactor -- -c ./examples/conf/server2.toml run
alias da := dev-demo-app
