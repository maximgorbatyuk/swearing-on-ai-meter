# CLAUDE.md

Guidance for Claude Code (and other agents) working in this repository.

## Project

`soaim` ("Swearing on AI Meter") is a Rust CLI/TUI that scans local AI-coding
prompt history across five apps — **Claude Code, Claude Desktop, Codex CLI,
Codex App, and opencode** — counts swear words, caches results in its own
SQLite database, and renders live terminal charts. Every statistic is also
broken down per app.

It is fully offline and read-only: it never writes to any tool's history.

## Commands

```sh
cargo build              # debug build
cargo build --release    # optimized binary at target/release/soaim
cargo run                # build + run (ingest, then open the TUI)
cargo test               # unit + integration tests
cargo clippy             # linter
cargo clippy -- -D warnings   # linter, treating warnings as errors (CI)
cargo fmt                # format
cargo fmt --check        # verify formatting without writing (CI)
```

Run a single test: `cargo test <name>` (e.g. `cargo test ingest_is_idempotent`).

### Running the binary

```sh
soaim                  # ingest new prompts, then open the dashboard
soaim ingest [--reset] # ingest only; --reset re-analyzes everything
soaim today            # today's count (no TUI)
soaim stats --days N   # window aggregates as text (no TUI)
soaim path             # resolved source + database paths
soaim discover         # inspect candidate source locations (use to locate
                       #   the Claude Desktop / Codex App prompt stores)
soaim --config <path>  # alternate config file
soaim --db <path>      # alternate database file
```

## Architecture

The binary (`src/main.rs`) is a thin shim over `soaim::run` in `src/lib.rs`;
the library crate exposes every module so `tests/` can exercise internals.

```
src/
  lib.rs         # run() — arg parsing + command dispatch
  cli.rs         # clap definitions
  config.rs      # load/merge TOML config + defaults, resolve all paths
  model.rs       # Source enum (the 5 apps) + RawPrompt
  detector.rs    # swear matching (whole-word, case-insensitive, total count)
  sources/       # one PromptSource impl per app, behind a common trait
    mod.rs         # trait + registry + shared parse helpers (ts, content, id)
    claude_code.rs codex_cli.rs opencode.rs      # implemented
    claude_desktop.rs codex_app.rs               # best-effort (see discover)
    discover.rs    # `soaim discover` diagnostic
  db.rs          # schema, migrations, dedupe inserts, hour_stat rollup, meta
  ingest.rs      # pipeline: read sources -> dedupe -> detect -> store (idempotent)
  stats.rs       # aggregation queries -> chart-ready structs (per-app aware)
  tui/           # ratatui dashboard
    mod.rs         # terminal setup + event loop
    app.rs         # state (window, filter, cached Dashboard)
    ui.rs          # layout + rendering of the panels
    widgets/       # custom heatmap + stacked per-app bar charts
tests/           # detector, source parsing (temp fixtures), db/ingest
```

### Key invariants

- **`analyzed_prompt` is authoritative**; `hour_stat` is an optional rollup
  cache. Stats queries read from `analyzed_prompt`.
- **Dedupe key** is `"{source}:{native_id}"`. Ingest only analyzes prompts not
  already stored, so it is incremental and idempotent.
- **All day/hour/weekday bucketing is in local time** (SQLite `localtime`).
- **`swear_count` is total occurrences** in a prompt (two swears => 2).
- Each `Source` maps to one of five fixed, ordered slots (`Source::index`),
  used for per-app series and consistent chart colors. Apps with no data render
  as zero, never missing.
- Sources are independent: a missing/uninstalled tool yields zero prompts and
  must never error, so the others keep working. Open external SQLite (opencode)
  **read-only**.

## Conventions

- `anyhow::Result` in app/binary code; prefer `?` over unwraps outside tests.
- SQLite is compiled in via `rusqlite`'s `bundled` feature — no system SQLite
  dependency. Keep it that way.
- Adding a new tool = a new `PromptSource` impl + a `Source` enum variant
  (update `Source::ALL`, `id`, `short`, `display`, `index`) + registry entry.
- The default swear list lives in `detector::default_words` and is overridable
  via `config.toml`.

## Notes

- The two desktop apps (Claude Desktop, Codex App) keep chat state in
  IndexedDB/LevelDB stores that aren't a stable prompt log. Run `soaim discover`
  on the target machine to locate the real store before wiring it up properly.
- TUI rendering targets the `ratatui` 0.29 API pinned in `Cargo.toml`.
