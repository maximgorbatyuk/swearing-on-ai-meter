mod common;

use std::path::PathBuf;

use rusqlite::Connection;
use soaim::model::Source;
use soaim::sources::{
    claude_code::ClaudeCode, codex_cli::CodexCli, file_changed_since, opencode::Opencode,
    PromptSource,
};

use common::{empty_config, write_file};

#[test]
fn claude_code_reads_only_user_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("claude/projects/p");
    // user prompt, assistant reply, a tool_result-only user turn (skipped),
    // and a user turn with text blocks.
    let jsonl = concat!(
        r#"{"type":"user","uuid":"u1","timestamp":"2025-01-02T10:00:00Z","message":{"role":"user","content":"please fix this fucking bug"}}"#,
        "\n",
        r#"{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":"sure"}}"#,
        "\n",
        r#"{"type":"user","uuid":"u2","timestamp":"2025-01-02T10:01:00Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}"#,
        "\n",
        r#"{"type":"user","uuid":"u3","timestamp":"2025-01-02T10:02:00Z","message":{"role":"user","content":[{"type":"text","text":"thanks"}]}}"#,
        "\n",
    );
    write_file(&root.join("session.jsonl"), jsonl);

    let mut cfg = empty_config(PathBuf::from("/tmp/unused.db"));
    cfg.paths.claude_code = dir.path().join("claude/projects");

    let prompts = ClaudeCode.read(&cfg, None).unwrap();
    assert_eq!(prompts.len(), 2, "tool_result-only and assistant skipped");
    assert!(prompts.iter().all(|p| p.source == Source::ClaudeCode));
    assert_eq!(prompts[0].native_id, "u1");
    assert!(prompts.iter().any(|p| p.text.contains("fucking bug")));
    assert!(prompts.iter().any(|p| p.text == "thanks"));
    assert!(prompts[0].created_at.is_some());
}

#[test]
fn codex_cli_reads_history_and_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("codex");
    write_file(
        &root.join("history.jsonl"),
        concat!(
            r#"{"ts":1735812000,"text":"damn it"}"#,
            "\n",
            r#"{"ts":1735812060,"text":""}"#,
            "\n",
        ),
    );
    write_file(
        &root.join("sessions/s1.jsonl"),
        concat!(
            r#"{"timestamp":"2025-01-02T11:00:00Z","role":"user","content":"shit happens"}"#,
            "\n",
            r#"{"role":"assistant","content":"indeed"}"#,
            "\n",
        ),
    );

    let mut cfg = empty_config(PathBuf::from("/tmp/unused.db"));
    cfg.paths.codex_cli = root;

    let prompts = CodexCli.read(&cfg, None).unwrap();
    assert_eq!(prompts.len(), 2);
    assert!(prompts.iter().any(|p| p.text == "damn it"));
    assert!(prompts.iter().any(|p| p.text == "shit happens"));
    assert!(prompts.iter().all(|p| p.source == Source::CodexCli));
}

#[test]
fn opencode_reads_user_messages_from_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("opencode.db");
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE message (id TEXT, time_created INTEGER, data TEXT);
             CREATE TABLE part (message_id TEXT, data TEXT);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO message VALUES ('m1', 1735812000000, '{\"role\":\"user\"}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO message VALUES ('m2', 1735812060000, '{\"role\":\"assistant\"}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO part VALUES ('m1', '{\"type\":\"text\",\"text\":\"this fucking thing\"}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO part VALUES ('m2', '{\"type\":\"text\",\"text\":\"assistant reply\"}')",
            [],
        )
        .unwrap();
    }

    let mut cfg = empty_config(PathBuf::from("/tmp/unused.db"));
    cfg.paths.opencode = db_path;

    let prompts = Opencode.read(&cfg, None).unwrap();
    assert_eq!(prompts.len(), 1, "only the user message");
    assert_eq!(prompts[0].native_id, "m1");
    assert!(prompts[0].text.contains("fucking thing"));
    assert!(prompts[0].created_at.is_some());
}

/// Helper to read a file's mtime as unix seconds.
fn mtime_secs(path: &std::path::Path) -> i64 {
    std::fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn file_changed_since_respects_watermark() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.jsonl");
    write_file(&f, "x");
    let mtime = mtime_secs(&f);

    assert!(file_changed_since(&f, None), "None always reads");
    assert!(
        file_changed_since(&f, Some(mtime - 10)),
        "past watermark -> changed"
    );
    assert!(
        !file_changed_since(&f, Some(mtime + 10)),
        "future watermark -> unchanged"
    );
    assert!(
        file_changed_since(&dir.path().join("missing.jsonl"), Some(mtime + 10)),
        "missing file -> read (correctness over speed)"
    );
}

#[test]
fn claude_code_skips_unchanged_files_via_watermark() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("claude/projects/p");
    let file = root.join("session.jsonl");
    write_file(
        &file,
        concat!(
            r#"{"type":"user","uuid":"u1","timestamp":"2025-01-02T10:00:00Z","message":{"role":"user","content":"hello"}}"#,
            "\n",
        ),
    );
    let mut cfg = empty_config(PathBuf::from("/tmp/unused.db"));
    cfg.paths.claude_code = dir.path().join("claude/projects");
    let mtime = mtime_secs(&file);

    // Watermark after the file's mtime -> file skipped, nothing read.
    assert_eq!(ClaudeCode.read(&cfg, Some(mtime + 10)).unwrap().len(), 0);
    // Watermark before the file's mtime -> file read.
    assert_eq!(ClaudeCode.read(&cfg, Some(mtime - 10)).unwrap().len(), 1);
    // No watermark -> read.
    assert_eq!(ClaudeCode.read(&cfg, None).unwrap().len(), 1);
}
