.PHONY: build build-release check test test-core test-desktop lint fmt clean install docs docs-rust docs-serve release deny typos machete

build:
	cargo build -p tome

build-release:
	cargo build -p tome --release

# Run from a clean isolated worktree at remote main; never switches main.
# Missing metadata creates a signed preparation PR and pauses for merge approval.
# Prepared metadata resumes at CI verification, signed tagging and asset checks.
# Homebrew publication remains a separate signed PR.
release:
ifndef VERSION
	$(error VERSION is required. Usage: make release VERSION=0.17.1)
endif
	python3 scripts/release.py --version '$(VERSION)'

release-test:
	python3 -B -m unittest discover -s scripts/tests -p 'test_release.py' -v

check:
	cargo check -p tome

test: test-core

# Fast default for CLI/core work; does not compile the Tauri desktop crate.
test-core:
	cargo test -p tome

# Run when changing the paused Tauri desktop crate or shared public APIs.
test-desktop:
	cargo test -p tome-desktop

lint:
	cargo clippy -p tome --all-targets -- -D warnings

fmt:
	cargo fmt -p tome

fmt-check:
	cargo fmt -p tome -- --check

clean:
	cargo clean

install:
	cargo install --path crates/tome

ci: fmt-check lint test typos
	@echo "All checks passed"

deny:
	cargo deny check

typos:
	typos

machete:
	cargo machete

docs: docs-rust
	mdbook build

docs-rust:
	cargo doc -p tome --no-deps

docs-serve:
	mdbook serve
