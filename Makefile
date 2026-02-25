.PHONY: build build-release check test lint fmt clean install docs docs-serve release

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
	gh pr create --title "Bump version to $$SEMVER" --body "Release $$TAG" --assignee MartinP7r; \
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
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

clean:
	cargo clean

install:
	cargo install --path crates/tome
	cargo install --path crates/tome-mcp

ci: fmt-check lint test
	@echo "All checks passed"

docs:
	mdbook build

docs-serve:
	mdbook serve
