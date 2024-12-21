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
