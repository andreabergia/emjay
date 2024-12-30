# https://github.com/casey/just

default: build test lint

build:
    cargo build

test:
    cargo nextest run

test-verbose:
    cargo nextest run --no-capture

lint:
    cargo clippy --fix --allow-dirty --allow-staged

clean:
    cargo clean

fmt:
    cargo fmt

coverage:
    rm \
        -rf \
        target/coverage-raw \
        target/coverage
    CARGO_INCREMENTAL=0 \
        RUSTFLAGS='-Cinstrument-coverage' \
        LLVM_PROFILE_FILE='target/coverage-raw/cargo-test-%p-%m.profraw' \
        cargo nextest run
    grcov \
        target/coverage-raw \
        --binary-path ./target/debug/deps/ \
        -s . \
        -t html \
        --branch \
        --ignore-not-existing \
        --ignore '../*' \
        --ignore "/*" \
        -o target/coverage/html

open-coverage:
    open target/coverage/html/index.html
