# githappens

> Git happens. Now you can see it.

A personal, terminal-native GitHub dashboard built in Rust with [`ratatui`](https://github.com/ratatui/ratatui). It lists your open GitHub pull requests in a TUI table with merge-readiness indicators, workflow status, and approval state.

## Features

- Lists all your open PRs across all repos in a single dashboard
- Color-coded merge-readiness: green (ready), yellow (waiting), red (failed)
- Workflow pass/total counts per PR
- Approval state indicators
- Press Enter to open a PR in your default browser
- Auto-refresh with configurable interval
- Rate-limit aware with countdown
- Token never logged, printed, or leaked

## Install

```sh
cargo install githappens
```

Or build from source:

```sh
git clone https://github.com/steffen-karlsson/githappens.git
cd githappens
cargo build --release
# Binary at target/release/githappens
```

## Usage

```sh
# Using an environment variable
export GIT_TOKEN=ghp_your_token_here
githappens

# Or passing the token directly
githappens --token ghp_your_token_here

# With options
githappens --token ghp_xxx --refresh 60 --max-prs 100
```

### CLI flags

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--token <T>` | `GIT_TOKEN` | — | GitHub PAT (required) |
| `--refresh <secs>` | — | `300` | Auto-refresh interval (min 30) |
| `--owner <login>` | — | token owner | Override "me" viewer |
| `--max-prs <N>` | — | `500` | Max PRs to fetch (hard cap 1000) |
| `--no-color` | `NO_COLOR` | `false` | Disable colored output |
| `--log-level <L>` | `RUST_LOG` | `info` | Log level (trace/debug/info/warn/error) |

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Enter` | Open selected PR in browser |
| `r` | Refresh |
| `R` | Force re-fetch |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Force quit |

## GitHub Token Setup

`githappens` needs a GitHub Personal Access Token (PAT) to read your open PRs,
their CI/check status, and review state. It only requires **read-only** access.

### Classic PAT

1. Go to https://github.com/settings/tokens/new
2. Select the `repo` scope (covers PRs, checks, and reviews for private repos)
   - If you only have public repos, `public_repo` is sufficient
3. Generate and copy the token
4. Set it as `GIT_TOKEN` in your environment or pass via `--token`

### Fine-grained PAT

1. Go to https://github.com/settings/personal-access-tokens/new
2. Grant these read-only permissions:
   - **Pull requests: Read-only**
   - **Actions: Read-only** (for check run status)
   - **Contents: Read-only** (for commit/rollup data)
3. Generate and copy the token
4. Set it as `GIT_TOKEN` in your environment or pass via `--token`

> **Security:** The token is never logged, printed in error messages, or
> included in panic payloads. A redaction layer scrubs it from all log output.

## Merge-readiness indicators

| Glyph | Color | State | Meaning |
|-------|-------|-------|---------|
| `●` | green | Ready | Mergeable, checks pass, approved, not draft |
| `◐` | yellow | Waiting | Pending checks, no approval, or mergeable unknown |
| `●` | red | Failed | Failed checks, changes requested, or conflicting |

Draft PRs are always yellow regardless of checks/approval. Repos without CI
are treated as having successful checks.

## Development

```sh
make build      # Compile
make test       # Run all tests
make lint       # fmt-check + clippy
make ci         # fmt-check + clippy + test + audit
```

## License

Dual-licensed under MIT or Apache-2.0 at your option.
