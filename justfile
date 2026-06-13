default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test

check:
    cargo check --all-targets

verify: fmt-check lint test check

build:
    cargo build --release

dist-plan:
    cargo dist plan
