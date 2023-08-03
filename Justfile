set dotenv-load

build: build-transactor build-cli

dep:
    cargo fetch
    npm --prefix ./js i -ws
    npm --prefix ./examples/demo-app i

build-facade:
    cargo build -r -p race-facade

build-transactor:
    cargo build -r -p race-transactor

build-cli:
    cargo build -r -p race-cli

cli *ARGS:
    cargo run -p race-cli -- {{ARGS}}

test: test-core test-transactor

test-transactor:
    cargo test -p race-transactor

test-core:
    cargo test -p race-core

examples: example-chat example-raffle

run-examples: examples preview-demo-app

example-minimal:
    cargo build -r -p race-example-minimal --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_minimal.wasm -o target/race_example_minimal.wasm

example-counter:
    cargo build -r -p race-example-counter --target wasm32-unknown-unknown

example-simple-settle:
    cargo build -r -p race-example-simple-settle --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_simple_settle.wasm -o target/race_example_simple_settle.wasm

example-chat:
    cargo build -r -p race-example-chat --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_chat.wasm -o target/race_example_chat.wasm

example-raffle:
    cargo build -r -p race-example-raffle --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_raffle.wasm -o target/race_example_raffle.wasm
    mkdir -p dev/dist
    cp target/race_example_raffle.wasm dev/dist/

example-draw-card:
    cargo build -r -p race-example-draw-card --target wasm32-unknown-unknown
    wasm-opt -Oz target/wasm32-unknown-unknown/release/race_example_draw_card.wasm -o target/race_example_draw_card.wasm
    mkdir -p dev/dist
    cp target/race_example_draw_card.wasm dev/dist/

dev-demo-app:
    npm --prefix ./examples/demo-app run dev

build-demo-app:
    npm --prefix ./examples/demo-app run build

preview-demo-app: build-demo-app
    npm --prefix ./examples/demo-app run preview

dev-facade *ARGS:
    cargo run -p race-facade -- {{ARGS}}

dev-reg-transactor conf:
    cargo run -p race-transactor -- -c {{conf}} reg

dev-transactor conf:
    cargo run -p race-transactor -- -c {{conf}} run

facade-transactor num:
    cargo run -p race-transactor -- -c examples/conf/server{{num}}.toml reg
    cargo run -p race-transactor -- -c examples/conf/server{{num}}.toml run

solana:
    (cd contracts/solana; cargo build-sbf)

solana-local: solana
    solana program deploy ./target/deploy/race_solana.so

borsh:
    npm --prefix ./js/borsh run build

sdk-core:
    npm --prefix ./js/sdk-core run build

sdk-solana:
    npm --prefix ./js/sdk-solana run build

sdk-facade:
    npm --prefix ./js/sdk-facade run build

sdk: borsh sdk-core sdk-solana sdk-facade

publish name url:
    cargo run -p race-cli -- -e local publish solana {{name}} {{url}}

create-reg:
    cargo run -p race-cli -- -e local create-reg solana

create-game spec:
    cargo run -p race-cli -- -e local create-game solana {{spec}}

validator:
    solana-test-validator --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s token_metadata_program.so

publish-npmjs pkg:
    npm --prefix ./js/{{pkg}} run build
    (cd js/{{pkg}}; npm publish --access=public)

publish-npmjs-all: (publish-npmjs "borsh") (publish-npmjs "sdk-core") (publish-npmjs "sdk-facade") (publish-npmjs "sdk-solana")

publish-crates pkg:
    cargo check -p {{pkg}}
    cargo test -p {{pkg}}
    cargo publish -p {{pkg}}

publish-crates-all: (publish-crates "race-env") (publish-crates "race-encryptor") (publish-crates "race-solana-types") (publish-crates "race-transport") (publish-crates "race-client") (publish-crates "race-test") (publish-crates "race-cli") (publish-crates "race-facade") (publish-crates "race-transactor")
