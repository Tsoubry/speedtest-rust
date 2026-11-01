default: release

fmt:
    cargo +nightly fmt

run:
    cargo run

build:
    cargo build

release:
    cargo build --release

check:
    cargo check

check-fmt:
    cargo +nightly fmt --all -- --check

clippy:
    cargo clippy -- -D warnings

test:
    cargo test -- --nocapture

check-all: check check-fmt clippy test
