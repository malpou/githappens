.PHONY: build run test lint fmt fmt-check clippy coverage audit ci clean help

build:
	cargo build

build-release:
	cargo build --release

run:
	cargo run

test:
	cargo test

lint: fmt-check clippy

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings

coverage:
	cargo llvm-cov --workspace --fail-under-lines 85

audit:
	cargo audit

ci: fmt-check clippy test audit

clean:
	cargo clean

help:
	@echo "Available targets:"
	@echo "  build          - Compile debug build"
	@echo "  build-release  - Compile release build"
	@echo "  run            - Run the application"
	@echo "  test           - Run all tests"
	@echo "  lint           - Run fmt-check + clippy"
	@echo "  fmt            - Format code"
	@echo "  fmt-check      - Check formatting"
	@echo "  clippy         - Run clippy with -D warnings"
	@echo "  coverage       - Run coverage (fail under 85%)"
	@echo "  audit          - Run cargo audit"
	@echo "  ci             - Run fmt-check + clippy + test + audit"
	@echo "  clean          - Clean build artifacts"
