set dotenv-load

# Release build transactor and command line tool
build: build-transactor build-cli

# Install NPM / Cargo dependencies
dep:
    cargo fetch
    npm --prefix ./js i -ws
    npm --prefix ./examples/demo-app i

# Release build facade server
build-facade:
    cargo build -r -p race-facade

# Release build transactor server
build-transactor:
    cargo build -r -p race-transactor

# Release build command line tool
build-cli:
    cargo build -r -p race-cli

# Call command line tool, use `just cli help` to show help menu
cli *ARGS:
    cargo run -q -p race-cli -- {{ARGS}}

# Run cargo test
test:
    cargo test
    npm test --prefix ./js

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

# Start demo app which serves the games in `examples`
dev-demo-app:
    npm --prefix ./examples/demo-app run dev

# Release build the demo-app
build-demo-app:
    npm --prefix ./examples/demo-app run build

# Release build the demo-app and open it in browser
preview-demo-app: build-demo-app
    npm --prefix ./examples/demo-app run preview

# Run facade with dev build, use `--help` to show help menu
dev-facade *ARGS:
    cargo run -p race-facade -- {{ARGS}}

dev-reg-transactor conf:
    cargo run -p race-transactor -- -c {{conf}} reg

dev-run-transactor conf:
    cargo run -p race-transactor -- -c {{conf}} run

# Start transactor dev build, read CONF configuration, register and run
dev-transactor conf: (dev-reg-transactor conf) (dev-run-transactor conf)

# Publish rust PKG to crates.io
publish-crates pkg:
    cargo check -p {{pkg}}
    cargo test -p {{pkg}}
    cargo publish -p {{pkg}}

# Publish all rust packages to crates.io
publish-crates-all: (publish-crates "race-api") (publish-crates "race-proc-macro") (publish-crates "race-core") (publish-crates "race-encryptor") (publish-crates "race-client") (publish-crates "race-test")
