.PHONY: build build-release check test lint fmt clean install docs docs-rust docs-serve release deny typos machete

build:
	cargo build

build-release:
	cargo build --release

# Usage: make release VERSION=0.1.3  (or VERSION=v0.1.3)
release:
ifndef VERSION
	$(error VERSION is required. Usage: make release VERSION=0.1.3)
endif
	@set -e; \
	SEMVER=$$(echo "$(VERSION)" | sed 's/^v//'); \
	TAG="v$$SEMVER"; \
	echo "Releasing $$TAG..."; \
	sed -i '' "s/^version = \".*\"/version = \"$$SEMVER\"/" Cargo.toml; \
	cargo check --quiet; \
	BRANCH="chore/release-$$TAG"; \
	git checkout -b "$$BRANCH"; \
	git commit --allow-empty -m "empty commit"; \
	git add Cargo.toml Cargo.lock; \
	git commit -m "Bump version to $$SEMVER"; \
	git push -u origin "$$BRANCH"; \
	gh pr create --head "$$BRANCH" --title "Bump version to $$SEMVER" --body "Release $$TAG" --assignee MartinP7r; \
	gh pr merge --squash --delete-branch; \
	git checkout main; \
	git pull origin main; \
	git tag "$$TAG"; \
	git push origin "$$TAG"; \
	echo "Released $$TAG — release workflow triggered"

check:
	cargo check

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

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
	cargo doc --no-deps

docs-serve:
	mdbook serve
