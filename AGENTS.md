# AGENTS.md

## Build & test commands

- **Build**: `make build` or `cargo build`
- **Run**: `make run` or `cargo run`
- **Test**: `make test` or `cargo test`
- **Lint**: `make lint` (runs fmt-check + clippy)
- **Format**: `make fmt` or `cargo fmt`
- **Format check**: `make fmt-check` or `cargo fmt --check`
- **Clippy**: `make clippy` or `cargo clippy -- -D warnings`
- **Coverage**: `make coverage` or `cargo llvm-cov --workspace --fail-under-lines 85`
- **Audit**: `make audit` or `cargo audit`
- **CI (all)**: `make ci` (fmt-check + clippy + test + audit)
- **Clean**: `make clean`

## Conventions

- Conventional commits for all commits.
- Clippy lints `unwrap_used`, `expect_used`, `dbg_macro`, `print_stdout`, `print_stderr` are denied in Cargo.toml.
- Minimum 85% line coverage enforced.
- No `unwrap()` or `expect()` in library code (use `?` or typed errors).
- Token must never appear in logs, errors, or panic payloads.
