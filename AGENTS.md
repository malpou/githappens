# AGENTS.md

## Project overview

`githappens` is a personal, terminal-native GitHub dashboard built in Rust with [`ratatui`](https://github.com/ratatui/ratatui). It lists your open GitHub pull requests in a TUI table with merge-readiness indicators, workflow status, and approval state.

- **Binary name**: `githappens`
- **Tagline**: *"Git happens. Now you can see it."*
- **PRD**: `docs/PRD.md`

## Build & test commands

- **Build**: `make build` or `cargo build`
- **Run**: `make run` or `cargo run`
- **Test**: `make test` or `cargo test`
- **Lint**: `make lint` (runs fmt-check + clippy)
- **Format**: `make fmt` or `cargo fmt`
- **Format check**: `make fmt-check` or `cargo fmt --check`
- **Clippy**: `make clippy` or `cargo clippy -- -D warnings`
- **Clippy (all targets)**: `cargo clippy --all-targets -- -D warnings`
- **Coverage**: `make coverage` or `cargo llvm-cov --workspace --fail-under-lines 85`
- **Audit**: `make audit` or `cargo audit`
- **CI (all)**: `make ci` (fmt-check + clippy + test + audit)
- **Clean**: `make clean`

## Architecture

```
src/
├── main.rs              # entrypoint, CLI parsing, panic hook, terminal setup
├── app.rs               # App struct, state machine, top-level run()
├── config.rs            # token resolution, CLI args, validation
├── log.rs               # tracing setup with token redaction layer
├── event.rs             # crossterm → typed KeyEvent; tick channel
├── browser.rs           # cross-platform open-URL
├── ui/
│   ├── mod.rs           # render entrypoint
│   ├── dashboard.rs     # the PR list table
│   ├── help_overlay.rs  # ? overlay
│   ├── error_screen.rs  # error/full-screen messages
│   └── theme.rs         # colors, glyphs (constant table)
├── github/
│   ├── mod.rs
│   ├── client.rs        # GraphQL POST client, pagination, retries
│   ├── query.graphql    # embedded via include_str!
│   ├── models.rs        # serde structs for GraphQL response (DTOs only)
│   └── pr.rs            # PullRequestSnapshot domain type
└── analysis/
    ├── mod.rs
    ├── mergeability.rs  # ready/waiting/failed state machine
    ├── workflows.rs     # count completed/total
    └── approval.rs      # collapse reviews → approval state
```

### Module responsibilities

- `config` — token/args only; no I/O.
- `log` — tracing setup + token redaction; never logs the token.
- `github::client` — HTTP transport only; no domain logic.
- `github::models` — DTOs only; no business rules.
- `github::pr` — pure translation DTO → domain `PullRequestSnapshot`.
- `analysis::*` — pure functions; no I/O. This is where 80% of tests live.
- `ui::*` — pure functions taking `&Frame` and `&App`. Side-effect-free.
- `app` — state machine; calls `github::client` via an injected trait `GitHubFetcher`.

### Trait-based seams for testing

`App::new(fetcher, config)` makes the entire app testable without HTTP. `main.rs` wires `HttpGitHubFetcher`. Tests use `MockGitHubFetcher`.

### State machine

States: `Loading`, `Ready`, `Refreshing`, `Error`, `RateLimited`, `Help`.

## Conventions

- Conventional commits for all commits.
- Clippy lints `unwrap_used`, `expect_used`, `dbg_macro`, `print_stdout`, `print_stderr` are denied in Cargo.toml.
- Minimum 85% line coverage enforced.
- No `unwrap()` or `expect()` in library code (use `?` or typed errors). `unwrap` is allowed in `#[cfg(test)]` modules via `#[allow(clippy::unwrap_used)]`.
- Token must never appear in logs, errors, or panic payloads. A `RedactingWriter` in `log.rs` scrubs the token from all log output.
- `anyhow` only at `main.rs`/CLI boundary; library code uses `thiserror` typed errors.
- `rustls-tls` (no openssl) for static binary and cross-platform builds.

## CLI flags

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--token <T>` | `GIT_TOKEN` | — | GitHub PAT (required) |
| `--refresh <secs>` | — | `300` | Auto-refresh interval (min 30) |
| `--owner <login>` | — | token owner | Override "me" viewer |
| `--max-prs <N>` | — | `500` | Max PRs to fetch (hard cap 1000) |

## GitHub API

- Uses **GraphQL v4** with a single batched query (one round-trip per page).
- Query is embedded via `include_str!("query.graphql")`.
- Endpoint: `https://api.github.com/graphql`
- Pagination via `hasNextPage` / `endCursor` up to `--max-prs` (hard stop at 1000).
- Retry once on 502/503/504 with 1s backoff. No retry on 401/403/422.
- HTTP timeout: 15s.
- Honors `X-RateLimit-Remaining` / `Retry-After`.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI rendering + terminal I/O |
| `tokio` | Async runtime |
| `reqwest` (rustls) | HTTP client for GraphQL |
| `serde` / `serde_json` | Serialization |
| `clap` | CLI argument parsing |
| `thiserror` | Typed errors in library code |
| `anyhow` | Top-level error context in main.rs |
| `webbrowser` | Open PR URLs in default browser |
| `tracing` + `tracing-subscriber` + `tracing-appender` | Structured logging with redaction |
| `async-trait` | `GitHubFetcher` trait |
| `directories` | Platform log directory resolution |

### Dev dependencies

| Crate | Purpose |
|-------|---------|
| `rstest` | Parametric tests for analysis modules |
| `wiremock` | Mock GraphQL server for client tests |
| `pretty_assertions` | Better assert_eq output |
| `assert_cmd` | CLI integration tests |
| `insta` | Snapshot tests for rendered UI |
