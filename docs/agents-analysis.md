# Where the stats data comes from

`soaim` reads **user-authored prompts** from each AI tool's local on-disk
history. It never reads agent/assistant replies, never calls a network, and
never writes to any tool's files. Stats are computed only from these prompts.

## Pipeline

`sources/*` read raw prompts → `ingest.rs` dedupes + counts swears →
`db.rs` stores rows in `analyzed_prompt` → `stats.rs` aggregates for charts.

- Dedupe key: `"{source}:{native_id}"`. Only new prompts get analyzed.
- `swear_count` = total swear occurrences in the prompt text (`detector.rs`,
  whole-word, case-insensitive).
- `analyzed_prompt` is authoritative; `hour_stat` is just a rollup cache.
- All stat queries read `analyzed_prompt`, bucketed in **local time**.

## Per-source data origins

| Source | Path (default) | Format | What counts as a prompt |
|--------|----------------|--------|--------------------------|
| **Claude Code** | `~/.claude/projects/**/*.jsonl` | JSONL | `type=="user"` and `message.role=="user"`; skip turns that are all `tool_result` |
| **Codex CLI** | `~/.codex/history.jsonl` + `~/.codex/**/*.jsonl` | JSONL | history: flat `text`/`prompt`/`input` field; rollouts: records with `role=="user"` |
| **opencode** | `~/.local/share/opencode/opencode.db` | SQLite (read-only) | `message` rows with `data.role=="user"`; text = concat of `part` rows where `type=="text"` |
| **Claude Desktop** | *(disabled until configured)* | JSONL best-effort | same rules as Claude Code |
| **Codex App** | *(disabled until configured)* | JSONL best-effort | same rules as Codex CLI |

### Notes per source

- **Claude Code / Codex CLI / opencode**: implemented and read by default.
- **Claude Desktop / Codex App**: their real store is IndexedDB/LevelDB, not a
  stable prompt log. They stay off until an explicit path is set in config; they
  then do a best-effort JSONL scan. Run `soaim discover` to locate the store.
- **opencode** is opened **read-only** so a running instance is never disturbed.
- Timestamps: epoch s/ms or ISO-8601 (`parse_ts`); opencode uses
  `time_created` epoch ms.
- `native_id` prefers a real id (uuid / message id), else a hash of
  `path|line` for stable dedupe.

## Field extraction

- Prompt text: `text_from_content` handles plain strings and content-block
  arrays (`type=="text"` blocks).
- Incremental reads: a per-source mtime watermark (`since`) skips unchanged
  files. Performance hint only — dedupe is the correctness backstop.
- A missing/uninstalled tool yields zero prompts and never errors, so the other
  sources keep working.
