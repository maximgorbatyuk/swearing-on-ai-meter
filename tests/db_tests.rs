mod common;

use chrono::Local;
use soaim::db::Db;
use soaim::ingest;
use soaim::model::Source;
use soaim::stats::{self, AppFilter, Window};

use common::{empty_config, write_file};

/// Insert directly, rebuild the rollup, and check the stats aggregation
/// (today, per-source, window total) with local-time bucketing.
#[test]
fn stats_aggregate_today_and_window() {
    let db = Db::open_in_memory().unwrap();
    let now = Local::now().timestamp();
    let five_days_ago = now - 5 * 86_400;

    {
        let tx = db.conn.unchecked_transaction().unwrap();
        Db::insert_prompt(&tx, "claude:1", "claude", "p", Some(now), now, 2).unwrap();
        Db::insert_prompt(&tx, "codex:1", "codex", "p", Some(now), now, 1).unwrap();
        Db::insert_prompt(
            &tx,
            "opencode:1",
            "opencode",
            "p",
            Some(five_days_ago),
            now,
            3,
        )
        .unwrap();
        // Duplicate id must be ignored.
        Db::insert_prompt(&tx, "claude:1", "claude", "p", Some(now), now, 99).unwrap();
        tx.commit().unwrap();
    }
    db.rebuild_hour_stat(now).unwrap();

    let dash = stats::dashboard(&db.conn, Window::D30, AppFilter::All).unwrap();
    assert_eq!(dash.today.total(), 3, "claude 2 + codex 1 today");
    assert_eq!(dash.today.per_source[Source::ClaudeCode.index()], 2);
    assert_eq!(dash.today.per_source[Source::CodexCli.index()], 1);
    assert_eq!(
        dash.window_total.iter().sum::<u32>(),
        6,
        "2 + 1 + 3 over the window"
    );

    // Single-app filter restricts to one source.
    let only_claude =
        stats::dashboard(&db.conn, Window::D30, AppFilter::One(Source::ClaudeCode)).unwrap();
    assert_eq!(only_claude.window_total.iter().sum::<u32>(), 2);
}

/// Full pipeline: ingest is incremental and idempotent.
#[test]
fn ingest_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("soaim.db");

    // A Claude Code fixture with two user prompts, one swearing.
    let projects = dir.path().join("claude/projects/p");
    write_file(
        &projects.join("s.jsonl"),
        concat!(
            r#"{"type":"user","uuid":"u1","timestamp":"2025-01-02T10:00:00Z","message":{"role":"user","content":"fix this damn thing"}}"#,
            "\n",
            r#"{"type":"user","uuid":"u2","timestamp":"2025-01-02T10:05:00Z","message":{"role":"user","content":"thanks"}}"#,
            "\n",
        ),
    );

    let mut cfg = empty_config(db_path.clone());
    cfg.paths.claude_code = dir.path().join("claude/projects");

    let mut db = Db::open(&db_path).unwrap();

    let first = ingest::run(&cfg, &mut db, false).unwrap();
    assert_eq!(first.new_per_source[Source::ClaudeCode.index()], 2);
    assert_eq!(first.swears_per_source[Source::ClaudeCode.index()], 1);

    let second = ingest::run(&cfg, &mut db, false).unwrap();
    assert_eq!(second.total_new(), 0, "re-running ingests nothing new");

    // The database holds exactly two analyzed prompts.
    let count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM analyzed_prompt", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

/// Incremental reads: an advanced per-source watermark skips that source's
/// unchanged files; rewinding the watermark re-reads them (deduped to 0 new).
#[test]
fn ingest_skips_unchanged_files_via_watermark() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("soaim.db");
    let projects = dir.path().join("claude/projects/p");
    write_file(
        &projects.join("s.jsonl"),
        concat!(
            r#"{"type":"user","uuid":"u1","timestamp":"2025-01-02T10:00:00Z","message":{"role":"user","content":"hello"}}"#,
            "\n",
            r#"{"type":"user","uuid":"u2","timestamp":"2025-01-02T10:05:00Z","message":{"role":"user","content":"there"}}"#,
            "\n",
        ),
    );

    let mut cfg = empty_config(db_path.clone());
    cfg.paths.claude_code = dir.path().join("claude/projects");
    let mut db = Db::open(&db_path).unwrap();
    let idx = Source::ClaudeCode.index();

    // First run has no watermark yet -> reads everything.
    let first = ingest::run(&cfg, &mut db, false).unwrap();
    assert_eq!(first.read_per_source[idx], 2, "first run reads all");

    // A watermark in the future makes the unchanged file skip the read entirely.
    let future = Local::now().timestamp() + 10_000;
    db.set_watermark("claude", future).unwrap();
    let skipped = ingest::run(&cfg, &mut db, false).unwrap();
    assert_eq!(skipped.read_per_source[idx], 0, "unchanged file skipped");
    assert_eq!(skipped.total_new(), 0);

    // Rewinding the watermark forces a re-read; id-dedupe keeps it at 0 new.
    db.set_watermark("claude", 0).unwrap();
    let reread = ingest::run(&cfg, &mut db, false).unwrap();
    assert_eq!(reread.read_per_source[idx], 2, "rewound watermark re-reads");
    assert_eq!(reread.total_new(), 0, "but dedupe stores nothing new");
}
