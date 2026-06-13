# Swearing on AI Meter (`soaim`)

A Rust CLI/TUI that scans your local AI-coding prompt history across five apps —
**Claude Code, Claude Desktop, Codex CLI, Codex App, and opencode** — counts
swear words, caches results in its own SQLite database, and shows live charts in
the terminal, with **every statistic also broken down per app**.

Everything runs fully offline and read-only: `soaim` never writes to any tool's
history.

## Install

### Homebrew

```sh
brew tap maximgorbatyuk/tap
brew install soaim
```

### Build from source

Requires a recent stable Rust toolchain (1.74+).

```sh
cargo build --release
./target/release/soaim
```

SQLite is compiled in (`rusqlite` `bundled` feature), so there is no system
SQLite dependency.

## Usage

```
soaim                  # ingest new prompts, then open the TUI dashboard
soaim ingest [--reset] # ingest only (cron-friendly); --reset re-analyzes everything
soaim today            # print today's swear count (no TUI)
soaim stats --days N   # print a window's aggregates as text (no TUI)
soaim path             # show resolved source paths + database path
soaim discover         # inspect candidate source locations and report findings
soaim --config <path>  # use an alternate config file
soaim --db <path>      # use an alternate database file
```

Running `soaim` with no arguments ingests any new prompts (incremental — only
new prompts are analyzed, so startup stays fast on large histories) and opens
the dashboard.

## Dashboard

* **Today counter** with a per-app breakdown.
* **GitHub-style daily heatmap** for the selected window.
* **Per-hour** and **per-weekday** bar charts, rendered as stacked per-app
  segments (each app a fixed color).
* All bucketing is in **local time**.

### Keys

| Key            | Action                              |
|----------------|-------------------------------------|
| `9` / `0`      | Switch window to 90 / 360 days      |
| `d` / `Esc`    | Back to the default 30-day window   |
| `a`            | Filter: all apps combined           |
| `1`–`5`        | Filter: claude / desktop / codex / codex-app / opencode |
| `r`            | Re-run ingest and refresh           |
| `q` / `Ctrl-C` | Quit                                |

## Sources

| Source         | Default location                        | Status |
|----------------|------------------------------------------|--------|
| Claude Code    | `~/.claude/projects/**/*.jsonl`          | implemented |
| Codex CLI      | `~/.codex/history.jsonl` + `~/.codex/**/*.jsonl` | implemented |
| opencode       | `~/.local/share/opencode/opencode.db`    | implemented (read-only SQLite) |
| Claude Desktop | `~/Library/Application Support/Claude`   | best-effort JSONL scan — run `soaim discover` to locate the real store |
| Codex App      | `~/Library/Application Support/Codex`    | best-effort JSONL scan — run `soaim discover` to locate the real store |

Claude Desktop and Codex App keep most chat state in IndexedDB/LevelDB stores
that are not a stable, documented prompt log. `soaim discover` inspects each
app's candidate directories on your machine — listing file counts, SQLite
schemas, and a sample message record — so the exact prompt store can be located
and wired up (the same way opencode was reverse-engineered). Until then those
two sources do a best-effort scan for JSONL chat logs and otherwise contribute
zero prompts; the other three keep working.

Any source path can be overridden in the config file or via `--config`.

## Configuration

A default config is created on first run at
`~/.config/soaim/config.toml` (Linux) /
`~/Library/Application Support/soaim/config.toml` (macOS):

```toml
# swears = ["fuck", "shit", "damn"]   # whole-word, case-insensitive

[paths]
# claude         = "~/.claude/projects"
# claude_desktop = "~/Library/Application Support/Claude"
# codex          = "~/.codex"
# codex_app      = "~/Library/Application Support/Codex"
# opencode       = "~/.local/share/opencode/opencode.db"

[db]
# path = "~/.local/share/soaim/soaim.db"
```

Precedence: CLI flags > config file > built-in defaults.

## How counting works

Matching is **case-insensitive** and **whole-word** (Unicode boundaries), so
`fucking` only matches if it's in the word list — it does not match `fuck`.
`swear_count` is the **total number of occurrences** in a prompt, so a prompt
with two swears counts 2.

## Development

```sh
cargo test          # unit + integration tests (detector, sources, db/ingest)
cargo clippy
cargo fmt
```

Tests build their inputs in temp directories (JSONL fixtures and an in-memory /
temp SQLite DB), so no fixture files are committed.

## Releasing

Distribution is handled by [`cargo-dist`](https://github.com/axodotdev/cargo-dist)
(pinned to `0.31.0`) + GitHub Actions. Pushing a `vX.Y.Z` tag triggers
[`.github/workflows/release.yml`](.github/workflows/release.yml), which:

1. Builds binaries for macOS (`aarch64`, `x86_64`), Linux (`x86_64`), and
   Windows (`x86_64`).
2. Generates installers — shell, PowerShell, Homebrew, and an MSI — plus
   checksums, and attaches them to a GitHub Release.
3. Publishes the `soaim` Homebrew formula to the
   [`maximgorbatyuk/homebrew-tap`](https://github.com/maximgorbatyuk/homebrew-tap)
   repo, which is what `brew install soaim` pulls.

The pipeline is configured in [`dist-workspace.toml`](dist-workspace.toml).

### Cutting a release

```sh
./scripts/release.py 0.2.0    # explicit version
./scripts/release.py          # auto-increment the patch from Cargo.toml
```

The script ([`scripts/release.py`](scripts/release.py)) automates a `dev` → `main`
flow: it validates the version, checks that [`CHANGELOG.md`](CHANGELOG.md) has a
matching latest entry, runs `just verify`, commits and pushes `dev`, merges into
`main`, then creates and pushes the `vX.Y.Z` tag that starts the release. Add the
new `## [X.Y.Z]` section to `CHANGELOG.md` (as the top entry) before running it.

Monitor the run with `gh run list --workflow release.yml`.

### Prerequisites

| Requirement | For | Install |
|-------------|-----|---------|
| `gh`, authenticated | `scripts/release.py` | `brew install gh` then `gh auth login` |
| `just` | `just verify` (release gate) | `brew install just` |
| `cargo-dist` 0.31.0 | reproducing the pipeline locally (`just dist-plan`) | `cargo install cargo-dist@0.31.0` |
| `HOMEBREW_TAP_TOKEN` repo secret | the Homebrew publish job | GitHub PAT with write access to `maximgorbatyuk/homebrew-tap`, added in repo Settings → Secrets |

On every push and pull request, [`.github/workflows/ci.yml`](.github/workflows/ci.yml)
runs the format check, Clippy (warnings as errors), build, and tests on Linux and
Windows.

## License

MIT — see `LICENSE`.
