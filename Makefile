.PHONY: build build-release check test lint fmt clean install docs docs-rust docs-serve release deny typos machete

build:
	cargo build

build-release:
	cargo build --release

# Usage: make release VERSION=0.1.3  (or VERSION=v0.1.3)
#
# FIX-06 (#533): the release recipe stamps the release date in CHANGELOG.md
# by replacing `## [Unreleased]` with `## [<SEMVER>] - <UTC date>`. Style
# matches the Cargo.toml version-bump sed line. Idempotent: if CHANGELOG.md
# lacks an `[Unreleased]` section (someone already cut a release without
# re-adding it), the sed substitution is a silent no-op and the release
# proceeds without the changelog edit.
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
	sed -i '' "s/^## \[Unreleased\]/## [$$SEMVER] - $$(date -u +%Y-%m-%d)/" CHANGELOG.md; \
	BRANCH="chore/release-$$TAG"; \
	git checkout -b "$$BRANCH"; \
	git commit --allow-empty -m "empty commit"; \
	git add Cargo.toml Cargo.lock CHANGELOG.md; \
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
