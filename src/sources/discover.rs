//! `soaim discover` — a diagnostic that inspects each source's candidate
//! locations and reports what it finds: file counts, SQLite schemas + row
//! counts, and a sample chat-message record. Ported from the Python PoC's
//! `--diagnose`. This is how the two not-yet-located stores (Claude Desktop,
//! Codex App) get reverse-engineered on the user's own machine.

use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::Config;
use crate::model::Source;
use crate::sources::{registry, text_from_content};

pub fn run(cfg: &Config) -> Result<()> {
    println!("=== soaim discover: where it looks and what it finds ===\n");

    for src in Source::ALL {
        println!("[{}]  {}", src.id(), src.display());
        match cfg.paths.for_source(src) {
            Some(root) => report_path(root)?,
            None => {
                // Not configured (desktop apps are disabled by default). Probe
                // the candidate locations to help locate the real store.
                let candidates = crate::config::candidate_dirs(src);
                if candidates.is_empty() {
                    println!("  -- no path resolved (store not located yet)");
                } else {
                    println!("  (not configured; probing candidate locations)");
                    for c in candidates {
                        report_path(&c)?;
                    }
                }
            }
        }
        println!();
    }

    println!("Implemented sources currently yield:");
    for src in registry() {
        let n = src.read(cfg, None).map(|v| v.len()).unwrap_or(0);
        println!("  {:<14} {:>6} prompts", src.name().id(), n);
    }
    Ok(())
}

fn report_path(root: &Path) -> Result<()> {
    let exists = root.exists();
    println!(
        "  {} {}  ({})",
        if exists { "OK" } else { "--" },
        root.display(),
        if exists { "exists" } else { "missing" }
    );
    if !exists {
        return Ok(());
    }

    // A direct file (e.g. opencode.db) vs a directory.
    if root.is_file() {
        if root.extension().is_some_and(|x| x == "db") {
            report_sqlite(root);
        }
        return Ok(());
    }

    // Directory: count files, surface any SQLite DBs, sample a message file.
    let mut jsonl = Vec::new();
    let mut json = Vec::new();
    let mut dbs = Vec::new();
    for e in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !e.file_type().is_file() {
            continue;
        }
        match e.path().extension().and_then(|x| x.to_str()) {
            Some("jsonl") => jsonl.push(e.path().to_path_buf()),
            Some("json") => json.push(e.path().to_path_buf()),
            Some("db") | Some("sqlite") | Some("sqlite3") => dbs.push(e.path().to_path_buf()),
            _ => {}
        }
    }
    println!(
        "       *.jsonl: {}   *.json: {}   *.db: {}",
        jsonl.len(),
        json.len(),
        dbs.len()
    );
    for db in &dbs {
        report_sqlite(db);
    }
    if let Some((path, rec)) = first_message_record(&jsonl, &json) {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        println!("       sample msg file: {}", rel.display());
        if let Some(obj) = rec.as_object() {
            let keys: Vec<&str> = obj.keys().take(12).map(String::as_str).collect();
            println!("         keys: {keys:?}");
        }
        let role = rec.get("role").and_then(Value::as_str).or_else(|| {
            rec.get("message")
                .and_then(|m| m.get("role"))
                .and_then(Value::as_str)
        });
        if let Some(role) = role {
            println!("         role: {role}");
        }
        let snippet_src = rec
            .get("content")
            .or_else(|| rec.get("parts"))
            .or_else(|| rec.get("text"))
            .cloned()
            .unwrap_or(Value::Null);
        let snippet: String = text_from_content(&snippet_src)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(80)
            .collect();
        if !snippet.is_empty() {
            println!("         text: \"{snippet}\"");
        }
    } else {
        println!("       (no chat-message-looking files found)");
    }
    Ok(())
}

fn report_sqlite(db: &Path) {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
    let Ok(conn) = Connection::open_with_flags(db, flags) else {
        println!("       SQLite DB present but unreadable: {}", db.display());
        return;
    };
    println!(
        "       SQLite DB: {}",
        db.file_name().unwrap_or_default().to_string_lossy()
    );
    if let Ok(mut stmt) =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
    {
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        for name in names {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |r| {
                    r.get(0)
                })
                .unwrap_or(-1);
            println!("         table {name}: {count} rows");
        }
    };
}

/// True if a record looks like a chat message or text part.
fn looks_like_message(rec: &Value) -> bool {
    if rec.get("role").is_some() {
        return true;
    }
    if rec.get("type").and_then(Value::as_str) == Some("text") && rec.get("text").is_some() {
        return true;
    }
    rec.get("message").and_then(|m| m.get("role")).is_some()
}

/// Find the first record across the given files that looks like a chat message,
/// so the sample is meaningful (not a config or lock file).
fn first_message_record(
    jsonl: &[std::path::PathBuf],
    json: &[std::path::PathBuf],
) -> Option<(std::path::PathBuf, Value)> {
    for path in jsonl {
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(rec) = serde_json::from_str::<Value>(line) {
                    if looks_like_message(&rec) {
                        return Some((path.clone(), rec));
                    }
                }
            }
        }
    }
    for path in json {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<Value>(&text) {
                let recs = match &val {
                    Value::Array(a) => a.clone(),
                    other => vec![other.clone()],
                };
                for rec in recs {
                    if looks_like_message(&rec) {
                        return Some((path.clone(), rec));
                    }
                }
            }
        }
    }
    None
}
