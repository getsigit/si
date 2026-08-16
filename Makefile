.PHONY: build test lint fmt check install clean

build:
	cargo build --workspace

test:
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

check: fmt lint test

install:
	cargo install --path crates/cli

clean:
	cargo clean
