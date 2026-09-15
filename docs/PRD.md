# PRD — `githappens` — Personal GitHub Dashboard (TUI)

**Chosen name:** `githappens`
**Binary name:** `githappens`
**Tagline:** *"Git happens. Now you can see it."*

---

## 1. Overview

`githappens` is a personal, terminal-native GitHub dashboard built in Rust with [`ratatui`](https://github.com/ratatui/ratatui). Its first release ships with one section: a live list of every open pull request you (the authenticated user) have authored, with at-a-glance merge-readiness indicators and one-key access to the PR in your browser.

### 1.1 Why
GitHub's PR inbox is noisy and browser-bound. Power users want a single keystroke-launchable view of *their own* open PRs and their CI/approval status, without leaving the terminal.

### 1.2 In one sentence
> `githappens` lists your open GitHub PRs in a TUI table, colored to tell you which can merge now, which are waiting on checks/approval, and which have failed — hit Enter to open any PR in the browser.

---

## 2. Goals / Non-goals

### Goals (v1)
- Launch from terminal, authenticate via token, render a single dashboard.
- Show every open PR authored by the authenticated user across all their repos.
- Display per-row merge-readiness, PR #, title, workflow pass/total, approval state.
- Open a PR in the default browser with Enter.
- Quit cleanly on `q` / `Ctrl+C` / `Esc`.
- Refresh on `r`.
- ≥ 85% line coverage with edge cases covered.

### Non-goals (v1)
- Multiple dashboards (issues, reviews requested, mentions, etc.) — reserved for v2 sections.
- Writing actions (merge, comment, approve) — v2.
- Org / team views — v2.
- OAuth device flow — v1 is PAT only.
- Notifications — v2.
- Persistent config file — v1 is env var + CLI flag only.
- Mouse support — keyboard only v1 (can be added in v2; ratatui supports it).

---

## 3. User stories

- **US-1**: As a dev, I run `githappens` with my PAT, so I can see all my open PRs without browsing.
- **US-2**: As a dev, I press `r` to refresh, so I don't have to relaunch when CI finishes.
- **US-3**: As a dev, I see a green/yellow/red glyph per PR so I can scan merge-readiness in 1 s.
- **US-4**: As a dev, I see workflow pass/total counts, so I know what's pending.
- **US-5**: As a dev, I see an approval bullet, so I know if I'm blocked on review.
- **US-6**: As a dev, I press Enter on a row, so the PR opens in my default browser.
- **US-7**: As a dev, I pass `--token` if I don't want to use `GIT_TOKEN`, so I can keep secrets in 1Password CLI / pass.
- **US-8**: As a dev, when the token is missing I get a clear error, not a stack trace.
- **US-9**: As a dev, when GitHub rate-limits me, the UI shows a clear "rate-limited, retry in Nm" message.
- **US-10**: As a dev, when I have zero open PRs, I see a friendly empty state, not a blank screen.

---

## 4. Functional requirements

### 4.1 Configuration & auth
| ID  | Requirement |
|-----|-------------|
| F-1 | Token resolution order: `--token <T>` flag > `$GIT_TOKEN` env var. Error if neither set. |
| F-2 | Token is never logged. Never printed in error messages. |
| F-3 | `--help` and `--version` follow clap defaults. |
| F-4 | Optional `--refresh <secs>` flag, default `300`. Min `30`. |
| F-5 | Optional `--owner <login>` flag to override the "me" viewer (default: token owner). |

### 4.2 GitHub data fetching
Use **GraphQL v4** with a single batched query (one round-trip per refresh).

- **F-10**: `viewer.login` and `viewer.pullRequests(states: OPEN, first: 100, orderBy: {field: UPDATED_AT, direction: DESC})`.
- **F-11**: For each PR, fetch: `number`, `title`, `url`, `headRefOid`, `mergeable`, `isDraft`, `commits(last:1).nodes[0].commit.statusCheckRollup`, `reviews(first:100)`.
- **F-12**: Pagination via `hasNextPage` / `endCursor` for users with >100 open PRs. Auto-paginate up to `--max-prs` (default 500). Hard stop at 1000.
- **F-13**: HTTP timeout 15s. Retry once on 502/503/504 with 1s backoff. No retry on 401/403/422.
- **F-14**: Honor `X-RateLimit-Remaining`/`Retry-After`; surface as UI state, do not silently swallow.
- **F-15**: HTTP 401/403 → "token invalid or expired"; HTTP 5xx → "GitHub unavailable, try again"; timeout → "request timed out".

### 4.3 Merge-readiness model (the indicator column)

Computed from the row's PR snapshot. The indicator is the *worst* of the constituent signals.

| State         | Conditions                                              | Color | Glyph |
|---------------|---------------------------------------------------------|-------|-------|
| **Ready**     | `mergeable == MERGEABLE` AND `statusCheckRollup.state == SUCCESS` AND approval == approved AND not draft | green  | `●` |
| **Waiting**   | Not failed, not ready. e.g. checks pending, or not approved, or `mergeable == UNKNOWN` | yellow | `◐` |
| **Failed**    | Any workflow `conclusion == FAILURE/TIMED_OUT/CANCELLED` OR approval == changes_requested OR `mergeable == CONFLICTING` | red    | `●` |

Edge cases encoded explicitly:
- **No workflows at all** → treated as `SUCCESS` for the workflow dimension (some repos have no CI). Documented in UI legend.
- **Draft PR** → always yellow (never ready), regardless of checks/approval.
- **`mergeable == UNKNOWN`** → yellow until GitHub computes.
- **Approval "needs review"** → yellow (not failed).

### 4.4 Workflow completion column
- Render as `completed / total`, e.g. `5/7`.
- `total` = count of nodes in `statusCheckRollup.contexts`.
- `completed` = count whose `status == COMPLETED` (CheckRun) or `state in (SUCCESS, ERROR, FAILURE)` (StatusContext).
- If `statusCheckRollup == null` → render `–/–` (en-dash).
- Width-capped: if total > 99, render `5+/99+`.

### 4.5 Approval column

Computed from `reviews` list, collapsed to latest review per author:

| Approval state | Definition                                              | Glyph | Color  |
|----------------|---------------------------------------------------------|-------|--------|
| Approved       | ≥1 latest review `APPROVED` and 0 `CHANGES_REQUESTED`  | `●`   | green  |
| Changes req.   | ≥1 latest review `CHANGES_REQUESTED`                    | `●`   | red    |
| Pending        | Latest reviews are all `PENDING`                        | `◔`   | yellow |
| None           | No reviews at all, or only `COMMENTED`/`DISMISSED`      | `○`   | gray   |

"Latest review per author" = sort reviews by `submittedAt` asc, then keep the last one per `author.login`. `DISMISSED` reviews are dropped before collapse.

### 4.6 UI layout (ratatui)

```
┌ githappens — ska's open PRs · refreshed 12s ago · r refresh · q quit ───┐
│ #   ▎ Title                              ▎ Checks ▎ Rev │ Link       │
│ ● 142  Add flaky CI retry               ▎ 7/7   ▎ ●     │            │
│ ◐ 138  Refactor webhooks                ▎ 4/6   ▎ ○     │            │
│ ●  99  Fix CVE-2024-1234                ▎ –/–   ▎ ●     │            │
│ ●  87  [Draft] Spike: Rust port          ▎ 2/2   ▎ ○     │            │
├────────────────────────────────────────────────────────┤
│ status: 5 open PRs · 2 ready · 1 failed · 0 rate-limited │
└──────────────────────────────────────────────────────────┘
```

- Header row sticky. Column widths via ratatui `Constraints` (Length: indicator 1+1, PR# 6, title flex, checks 8, review 3).
- Row selection highlighted (reverse video).
- Title column truncates with `…` if longer than column.
- Footer shows counts + refresh state + rate-limit state.

### 4.7 Keybindings
| Key       | Action                              |
|-----------|-------------------------------------|
| `j`/`↓`   | Move selection down                 |
| `k`/`↑`   | Move selection up                   |
| `g`       | Top                                 |
| `G`       | Bottom                              |
| `Enter`   | Open selected PR in default browser |
| `r`       | Manual refresh                      |
| `R`       | Refresh + force re-fetch (clear cache) |
| `?`       | Toggle help overlay                 |
| `q`/`Esc` | Quit                                |
| `Ctrl+C`  | Force quit                          |

### 4.8 Non-functional requirements
- **N-1**: Cold-start to first render < 1.5s on a 50-PR account on a normal connection.
- **N-2**: Render loop ≤ 4 fps; redraw only on event.
- **N-3**: No panics on malformed GitHub responses; all `Result`-typed functions.
- **N-4**: Token never in logs, never in error strings, never in panic payloads.
- **N-5**: Clean terminal restore on any exit path (panic handler installed).
- **N-6**: Works on macOS Terminal, iTerm2, Alacritty, Kitty, WezTerm, Windows Terminal.
- **N-7**: Single static binary, no runtime deps; `cargo install`-able.

---

## 5. Architecture

### 5.1 Crate layout

```
githappens/
├── Cargo.toml
├── README.md
├── AGENTS.md                      # build/test/lint commands for tooling
├── docs/
│   └── PRD.md                     # this file
├── src/
│   ├── main.rs                    # entrypoint, CLI parsing, panic hook, terminal setup
│   ├── app.rs                     # App struct, state machine, top-level run()
│   ├── config.rs                  # token resolution, CLI args, validation
│   ├── event.rs                   # crossterm → typed KeyEvent; tick channel
│   ├── ui/
│   │   ├── mod.rs                 # render entrypoint
│   │   ├── dashboard.rs           # the PR list table
│   │   ├── help_overlay.rs        # ? overlay
│   │   ├── error_screen.rs        # error/full-screen messages
│   │   └── theme.rs               # colors, glyphs (constant table)
│   ├── github/
│   │   ├── mod.rs
│   │   ├── client.rs              # GraphQL POST client, pagination, retries
│   │   ├── query.graphql          # embedded via include_str!
│   │   ├── models.rs              # serde structs for GraphQL response
│   │   └── pr.rs                  # PullRequestSnapshot domain type
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── mergeability.rs        # ready/waiting/failed state machine
│   │   ├── workflows.rs           # count completed/total
│   │   └── approval.rs            # collapse reviews → approval state
│   └── browser.rs                 # cross-platform open-URL
├── tests/
│   ├── common/
│   │   └── mod.rs                 # fixtures loader, mock server helpers
│   ├── integration/
│   │   ├── client.rs              # HTTP layer (mocked)
│   │   ├── pagination.rs
│   │   ├── rate_limit.rs
│   │   ├── error_codes.rs
│   │   └── end_to_end.rs          # spin mock server + ratatui TestBackend
│   └── fixtures/
│       ├── one_open_pr.json
│       ├── many_prs_page1.json
│       ├── many_prs_page2.json
│       ├── rate_limited.json
│       ├── drafts_and_conflicts.json
│       └── malformed.json
```

### 5.2 Module responsibilities (single-responsibility per file)

- `config` — token/args only; no I/O.
- `github::client` — HTTP transport only; no domain logic.
- `github::models` — DTOs only; no business rules.
- `github::pr` — pure translation DTO → domain `PullRequestSnapshot`.
- `analysis::*` — pure functions; no I/O. **This is where 80% of tests live.**
- `ui::*` — pure functions taking `&Frame` and `&App`. Side-effect-free.
- `app` — state machine; calls `github::client` via an injected trait `GitHubFetcher`.

### 5.3 Trait-based seams for testing

```rust
#[async_trait::async_trait]
pub trait GitHubFetcher: Send + Sync {
    async fn fetch_open_prs(&self, owner: &str, max: usize) -> Result<FetchOutcome>;
}

pub struct HttpGitHubFetcher { /* reqwest client, token */ }
pub struct MockGitHubFetcher { /* scripted responses */ }
```

`App::new(fetcher, config)` makes the entire app testable without HTTP, while `main.rs` wires `HttpGitHubFetcher`.

### 5.4 State machine

```
                ┌────────────┐
   launch ────▶ │  Loading   │──┐
                └────────────┘  │ ok  ┌────────────┐
                                ├────▶│  Ready      │◀─┐
                                │     └────────────┘  │
                                │ err                │ refresh
                                ▼                    │
                          ┌────────────┐              │
                          │   Error    │──── r ───────┘
                          └────────────┘
```

States: `Loading`, `Ready`, `Refreshing`, `Error`, `RateLimited`, `Help`.
Sub-states of Ready: empty (no PRs), populated.

### 5.5 Data flow

```
main ─▶ config::resolve_token() ─▶ HttpGitHubFetcher::new(token)
      ─▶ App::new(fetcher, cfg)
      ─▶ App::run()
              ├── event loop (tokio + crossterm)
              ├── on tick/refresh: fetcher.fetch_open_prs() ─▶ Vec<PullRequestSnapshot>
              ├── analysis::* applied to each snapshot ─▶ Vec<RowView>
              └── ui::draw(frame, &app_state) ─▶ ratatui
```

---

## 6. Test strategy (target ≥ 85% line coverage)

### 6.1 Test pyramid

| Layer           | Tool                                | Coverage focus |
|-----------------|-------------------------------------|----------------|
| Pure logic      | `#[test]` + `rstest`                | mergeability, workflow count, approval collapse — **~95%** |
| Parsing         | `#[test]` over `tests/fixtures/*.json` | model serde + DTO→domain |
| HTTP client     | `wiremock` (mock GraphQL)           | pagination, retries, rate limits, error codes |
| UI rendering    | ratatui `TestBackend`               | row count, headers, colors, truncation, empty state |
| End-to-end      | `TestBackend` + `MockGitHubFetcher` | key → state transition, Enter → URL opened |
| CLI             | `assert_cmd` + `clap`              | `--help`, `--version`, missing token error |

### 6.2 Required test cases (edge-case coverage)

#### 6.2.1 `analysis::mergeability`
- All-green PR → Ready.
- Draft PR with all-green → Waiting (never Ready).
- `mergeable == UNKNOWN` → Waiting.
- `mergeable == CONFLICTING` → Failed (red).
- Checks SUCCESS but no approval → Waiting.
- Checks PENDING → Waiting.
- Checks FAILURE → Failed.
- Checks null/absent (no CI) → treated as SUCCESS (Ready if approval green).
- Mixed: FAILURE on one check + APPROVED → Failed.
- Mixed: PENDING + CHANGES_REQUESTED → Failed (changes drives failure).
- Mixed: PENDING + no reviews → Waiting.

#### 6.2.2 `analysis::workflows`
- 7/7 completed → "7/7".
- 5/7 with 2 pending → "5/7".
- 0/0 (null rollup) → "–/–".
- 100/100 → "99+/99+" (cap).
- Mixed CheckRun + StatusContext both counted.
- StatusContext with `state == PENDING` → not completed.
- CheckRun with `status == COMPLETED`, `conclusion == null` → not counted as completed (still "running" semantics) — *documented decision*.

#### 6.2.3 `analysis::approval`
- Single APPROVED → Approved.
- Single CHANGES_REQUESTED → ChangesRequested.
- APPROVED then later CHANGES_REQUESTED by same author → ChangesRequested.
- CHANGES_REQUESTED then later APPROVED by same author → Approved (last wins).
- Two authors: one APPROVED, one CHANGES_REQUESTED → ChangesRequested.
- Two authors: both APPROVED → Approved.
- Only COMMENTED → None.
- DISMISSED reviews dropped before collapse.
- Empty reviews list → None.
- PENDING review present, no others → Pending.
- Mixed PENDING + APPROVED by different authors → Approved (PENDING is informational).
- Reviews with null `author.login` (bots) → still collapse by `author.login.unwrap_or("")`.

#### 6.2.4 `github::client` (via `wiremock`)
- 200 with single page → returns all PRs.
- 200 with `hasNextPage` → auto-paginates, concatenates, dedupes by `number`.
- Pagination hits `--max-prs` → stops gracefully.
- Pagination hits hard 1000 cap → stops, sets `truncated: true` flag in `FetchOutcome`.
- HTTP 401 → `Err(TokenInvalid)`.
- HTTP 403 with `X-RateLimit-Remaining: 0` → `Err(RateLimited { retry_after })`.
- HTTP 503 once then 200 → retried, success.
- HTTP 503 twice → `Err(GitHubUnavailable)`.
- 15s timeout → `Err(Timeout)`.
- Malformed JSON → `Err(Parse(_))` no panic.
- Missing fields in JSON (e.g. `statusCheckRollup: null`) → handled, not panic.
- Token header is set on every request; token never in URL/query.
- Token never echoed in any `Err` Display impl (regression test).

#### 6.2.5 UI rendering (`TestBackend`)
- 0 PRs → empty state screen rendered, no rows.
- 1 PR → 1 data row + header + footer.
- 50 PRs → 50 rows.
- Long title (200 chars) → truncated with `…` at column width.
- Empty title `""` → renders empty cell, no panic.
- Unicode title (emoji, CJK) → rendered via `Line::from`, no panic, width ≈ display width.
- Selection highlight visible on first row.
- Selection moves with `j`/`k`.
- Selection wraps (or clamps) at top/bottom — *clamp* is the chosen behavior.
- Footer counts: N total, X ready, Y failed, Z rate-limited (0 normally).
- Help overlay toggled with `?` hides table.
- Error screen rendered when state == Error.

#### 6.2.6 Keybindings & state
- `q`/`Esc` → App exits with `ExitCode::SUCCESS`.
- `Ctrl+C` → App exits with `ExitCode::from(130)`.
- `Enter` on row → `webbrowser::open(url)` called exactly once with the PR URL.
- `Enter` in empty state → no-op (no panic).
- `r` triggers refresh; state transitions `Ready → Refreshing → Ready`.
- `r` while Refreshing → ignored (debounced).
- `R` clears cache (forces re-fetch).
- Arrow beyond last row → clamp (no panic).
- Refresh after rate-limit error returns to Ready on success.

#### 6.2.7 Config & CLI
- `--token` provided → used.
- `GIT_TOKEN` env only → used.
- Both → `--token` wins.
- Neither → non-zero exit, stderr message "error: no GitHub token provided (set GIT_TOKEN or pass --token)".
- `--refresh 10` → rejected with "must be ≥ 30".
- `--refresh -5` → rejected.
- `--owner foo` → forwarded to fetcher.
- `--help` → exit 0, contains "GIT_TOKEN".
- `--version` → exit 0, contains crate version.
- Token passed via flag is not echoed back in `--help` or any error.

#### 6.2.8 Non-functional regressions
- Panic-hook test: a simulated panic in draw restores terminal (capture raw stdout).
- No `unwrap()` in hot path (audit via `clippy::unwrap_used` and `clippy::expect_used` lints, deny in CI).
- `cargo audit` passes.
- `cargo fmt --check` passes.
- `cargo clippy -- -D warnings` passes.

### 6.3 Coverage gating

```toml
# Cargo.toml (dev-dependencies section)
[dev-dependencies]
rstest = "0.x"
wiremock = "0.x"
pretty_assertions = "0.x"
assert_cmd = "2.x"
insta = "1.x"          # snapshot tests for rendered UI
ratatui = { version = "0.x", features = ["crossterm"] }
criterion = "0.x"     # bench (optional, not in coverage gate)

[profile.coverage]
inherits = "test"
```

CI runs (Tarpaulin or `cargo-llvm-cov`):
```bash
cargo llvm-cov --workspace --fail-under-lines 85 --features test-utils
```

Coverage **must not drop below 85%** or CI fails. Target: 90% on `analysis/` and `github/models`, 80% on `ui/` (terminal rendering is hard to fully cover), 100% on `config.rs`.

---

## 7. Dependencies

```toml
[dependencies]
ratatui = { version = "0.x", features = ["crossterm"] }
crossterm = "0.x"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.x", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive", "env", "wrap_help"] }
anyhow = "1"                # top-level error context only
thiserror = "1"             # typed errors
webbrowser = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"    # rotating file logs (NEVER logs token; tested)
async-trait = "0.1"
directories = "5"           # log dir resolution
```

Justifications:
- `rustls-tls` (no openssl) keeps static binary, simplifies macOS/Windows builds.
- `anyhow` only at `main.rs`/CLI boundary; library code uses `thiserror` typed errors.
- `tracing` (not `log`) for structured logs; `tracing-appender` writes to platform log dir.

---

## 8. GraphQL query (single round-trip per page)

```graphql
query OpenPRs($owner: String!, $first: Int!, $after: String) {
  viewer {
    login
    pullRequests(states: OPEN, first: $first, after: $after,
      orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        title
        url
        isDraft
        mergeable
        headRefOid
        repository { nameWithOwner }
        commits(last: 1) {
          nodes {
            commit {
              statusCheckRollup {
                state
                contexts {
                  __typename
                  ... on CheckRun {
                    name
                    status
                    conclusion
                  }
                  ... on StatusContext {
                    name
                    state
                  }
                }
              }
            }
          }
        }
        reviews(first: 100) {
          nodes {
            author { login }
            state
            submittedAt
          }
        }
      }
    }
  }
}
```

Note: GraphQL `viewer` always returns the token owner; `--owner` flag requires switching to `user(login:)` field. We query `user(login: $owner)` when `--owner != viewer.login`. Documented in `github/client.rs`.

---

## 9. Incremental development plan

Each phase ends with a green CI build, an updated CHANGELOG entry, and a git tag.

### Phase 0 — Skeleton (½ day)
- `cargo new --bin githappens`, set up `Cargo.toml` deps.
- Add `clap` parser: `--token`, `--refresh`, `--owner`, `--version`.
- Add `config::resolve_token` with env fallback; tests §6.2.7.
- Add AGENTS.md with `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo test`, `cargo llvm-cov --fail-under-lines 85`.
- Set up `tracing` + `tracing-appender`, plus a redaction test (F-2/N-4).
- **Exit criteria**: `cargo run -- --help` works, missing-token error works, CI green, coverage ≥ 85% (trivially, since little code).

### Phase 1 — GitHub client (1 day)
- Implement `GitHubFetcher` trait + `HttpGitHubFetcher`.
- Embed `query.graphql`; send POST to `https://api.github.com/graphql`.
- Pagination loop with `--max-prs` cap and hard 1000 stop.
- Retry on 5xx (one retry, 1s backoff), error mapping per F-15.
- Tests §6.2.4 using `wiremock`; fixtures in `tests/fixtures/`.
- **Exit criteria**: `cargo test --test client` passes; pagination test asserts 2-page concat + dedup; rate-limit test asserts retry-after surfaced.

### Phase 2 — Domain & analysis (1 day)
- `PullRequestSnapshot` domain type (no serde derives — hand-translated from DTOs).
- `analysis::mergeability`, `analysis::workflows`, `analysis::approval` — all pure.
- Tests §6.2.1, §6.2.2, §6.2.3 with `rstest` parametric tables.
- Snapshot tests (`insta`) for each row's computed `RowView`.
- **Exit criteria**: `cargo test analysis` green; `cargo llvm-cov` on `analysis/` ≥ 95%.

### Phase 3 — TUI scaffold (1 day)
- `crossterm` raw mode + alternate screen + panic hook (restore on panic).
- `event.rs` async channel with 250ms tick for refresh scheduling.
- `App` state machine (Loading/Ready/Refreshing/Error/RateLimited/Help).
- `ui::dashboard` renders the table; `ui::theme` defines colors/glyphs.
- `TestBackend` tests §6.2.5 — use `insta` snapshots of rendered buffer.
- **Exit criteria**: `cargo run` shows your PRs; `Enter` is a no-op; `q` quits cleanly; terminal restored after panic test.

### Phase 4 — Interactions (½ day)
- `j/k/g/G/Enter/r/R/?/q/Esc/Ctrl+C` handling per §4.7.
- `webbrowser::open` on Enter with the PR's `url`.
- State-transition tests §6.2.6 using `MockGitHubFetcher`.
- Empty-state screen ("You have no open PRs.  Go open one 🎉").
- **Exit criteria**: full keyboard control works; Enter opens browser exactly once.

### Phase 5 — Refresh & rate-limit UX (½ day)
- Auto-refresh via `--refresh` interval.
- Debounce manual `r` while Refreshing.
- Rate-limited screen showing retry-after countdown.
- Footer counts (total / ready / failed / rate-limited).
- Tests: `tokio::time::pause` for timers; `MockGitHubFetcher` returns `RateLimited` once then `Ready`.
- **Exit criteria**: rate-limit UI renders countdown; refresh after recovery returns to Ready.

### Phase 6 — Polish & release (1 day)
- README with install/usage/screenshots (`svg-term` cast → SVG).
- `cargo dist` or plain `cargo build --release` CI matrix (macOS arm/x86, Linux, Windows).
- `cargo audit` in CI.
- Add `--no-color` flag (respects `NO_COLOR` env per no-color.org).
- Add `--log-level` flag.
- License (MIT/Apache-2.0 dual), CODE_OF_CONDUCT, CONTRIBUTING.
- Tag `v0.1.0`; publish to crates.io if desired.
- **Exit criteria**: v0.1.0 binary on GitHub Releases for 4 targets; `cargo install githappens` works.

### Phase 7 (post-v1, parked) — v2 backlog
- Sections: Reviews Requested, Mentions, Assigned Issues, Notifications.
- Write actions: approve, comment, merge (with confirmation modal).
- Config file (`githappens.toml`) for default owner/refresh/filter.
- OAuth device flow.
- Mouse selection + click-to-open.
- Filter / search within list (`/`).
- Sort by column.

---

## 10. Open questions

1. **Mergeable freshness** — `mergeable` can be `UNKNOWN` for up to a few seconds after a push. Should we auto-re-fetch 5s after a first UNKNOWN? *Proposed: yes, with a max 2 re-fetches.*
2. **Approvals from Bots** — should `github-actions` bot approvals count? *Proposed: no — drop reviews whose author login ends in `[bot]`.*
3. **Multiple accounts** — out of scope v1; record here for v2.
4. **Required-status semantics** — `statusCheckRollup.state == FAILURE` is GitHub's own rollup; do we trust it, or recompute from contexts? *Proposed: trust GitHub's rollup for the mergeability indicator, but render the explicit `completed/total` count from contexts.*

---

## 11. Success metrics (v1)

- Cold start to first paint < 1.5s on a 50-PR account (N-1).
- Zero panics over a 14-day dogfooding period.
- 85% line coverage floor enforced in CI.
- Binary size < 8 MB (release, stripped).
- One command to install: `cargo install githappens`.

---

## 12. Risks

| Risk | Mitigation |
|------|-----------|
| GitHub GraphQL schema changes | Pin to a documented schema version; pin in tests; snapshot fixture tests catch drift. |
| Token leak in logs | Redaction layer in `tracing` subscriber + regression test that searches log output for the token string. |
| Ratatui breaking changes | Pin minor version; bump in dedicated PR with snapshot regeneration. |
| Rate limits on large accounts | Auto-pagination + caching; surface `Retry-After`; warn before fetch if last rate-limit was < 100. |
| Terminal restoration on panic | Panic hook restores terminal before printing panic; covered by regression test. |
