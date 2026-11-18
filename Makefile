.DEFAULT_GOAL := help

.PHONY: help run demo fmt fmt-check lint lint-fix test review check build build-release clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-16s\033[0m %s\n", $$1, $$2}'

run: ## Run wifiess normally
	cargo run

demo: ## Run wifiess against the demo backend
	cargo run -- --demo

fmt: ## Format Rust code
	cargo fmt --all

fmt-check: ## Check code formatting without modifying files
	cargo fmt --all -- --check

lint: ## Run clippy linter with strict warnings
	cargo clippy --all-targets -- -D warnings

lint-fix: ## Automatically fix clippy suggestions where possible
	cargo clippy --fix --allow-dirty --allow-staged --all-targets

test: ## Run unit and UI snapshot tests
	cargo test --all-targets

review: ## Review changed insta snapshots
	cargo insta review

check: fmt-check lint test ## Run all checks (fmt, clippy, test) matching CI

build: ## Build debug binary
	cargo build

build-release: ## Build optimized release binary
	cargo build --release

clean: ## Clean cargo target artifacts
	cargo clean
