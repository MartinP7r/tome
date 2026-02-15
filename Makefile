.PHONY: build release check test lint fmt clean install

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

clean:
	cargo clean

install:
	cargo install --path crates/skync
	cargo install --path crates/skync-mcp

ci: fmt-check lint test
	@echo "All checks passed"
