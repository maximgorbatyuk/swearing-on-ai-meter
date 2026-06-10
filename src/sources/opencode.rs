//! opencode: `~/.local/share/opencode/opencode.db` (SQLite).
//!
//! Opened **read-only** so a running opencode is never disturbed. A user
//! prompt is a `message` row with `data.role == "user"`; its text is the
//! concatenation of its `part` rows where `data.type == "text"`. Timestamps
//! come from `message.time_created` (epoch ms). `native_id` is the message id.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::{parse_epoch_millis, PromptSource};
use crate::config::Config;
use crate::model::{RawPrompt, Source};

pub struct Opencode;

impl PromptSource for Opencode {
    fn name(&self) -> Source {
        Source::Opencode
    }

    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>> {
        let db = &cfg.paths.opencode;
        if !db.exists() {
            return Ok(Vec::new());
        }
        // Incremental skip: a new opencode message bumps the mtime of either
        // the DB file or its `-wal` sidecar (writes land in the WAL before a
        // checkpoint), so skip only when neither has changed since `since`.
        if let Some(since) = since {
            if db_unchanged_since(db, since) {
                return Ok(Vec::new());
            }
        }
        let Some(conn) = open_readonly(db) else {
            return Ok(Vec::new());
        };
        read_prompts(&conn)
    }
}

/// True when the newest mtime among the DB file and its `-wal` sidecar is
/// strictly before `since`. A missing sidecar is ignored; if nothing can be
/// stat'd, returns `false` so the read still happens (correctness over speed).
fn db_unchanged_since(db: &Path, since: i64) -> bool {
    let wal = PathBuf::from(format!("{}-wal", db.display()));
    let newest = [db.to_path_buf(), wal]
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .filter_map(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .max();
    match newest {
        Some(m) => m < since,
        None => false,
    }
}

/// Open `db` read-only. Tries plain read-only mode, then a URI `immutable=1`
/// fallback (matching the PoC). Returns `None` if it cannot be opened.
fn open_readonly(db: &std::path::Path) -> Option<Connection> {
    let ro = OpenFlags::SQLITE_OPEN_READ_ONLY;
    if let Ok(c) = Connection::open_with_flags(db, ro) {
        return Some(c);
    }
    let uri = format!("file:{}?immutable=1", db.to_string_lossy());
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    Connection::open_with_flags(uri, flags).ok()
}

fn read_prompts(conn: &Connection) -> Result<Vec<RawPrompt>> {
    // message_id -> time_created (epoch ms), only for user messages.
    let mut user_msgs: HashMap<String, Option<i64>> = HashMap::new();
    {
        let mut stmt = match conn.prepare("SELECT id, time_created, data FROM message") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()), // unexpected schema; yield nothing
        };
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let time_created: Option<i64> = row.get(1).ok();
            let data: Option<String> = row.get(2).ok();
            Ok((id, time_created, data))
        })?;
        for row in rows {
            let (id, time_created, data) = row?;
            let Some(data) = data else { continue };
            let Ok(d) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            if d.get("role").and_then(Value::as_str) == Some("user") {
                user_msgs.insert(id, time_created);
            }
        }
    }

    if user_msgs.is_empty() {
        return Ok(Vec::new());
    }

    // message_id -> concatenated text parts (in row order).
    let mut texts: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut stmt = match conn.prepare("SELECT message_id, data FROM part") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt.query_map([], |row| {
            let mid: String = row.get(0)?;
            let data: Option<String> = row.get(1).ok();
            Ok((mid, data))
        })?;
        for row in rows {
            let (mid, data) = row?;
            if !user_msgs.contains_key(&mid) {
                continue;
            }
            let Some(data) = data else { continue };
            let Ok(d) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            if d.get("type").and_then(Value::as_str) == Some("text") {
                if let Some(t) = d.get("text").and_then(Value::as_str) {
                    if !t.is_empty() {
                        texts.entry(mid).or_default().push(t.to_string());
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for (mid, time_created) in user_msgs {
        let prompt = texts.get(&mid).map(|v| v.join("\n")).unwrap_or_default();
        if prompt.trim().is_empty() {
            continue;
        }
        let created_at = time_created.and_then(parse_epoch_millis);
        out.push(RawPrompt {
            source: Source::Opencode,
            native_id: mid,
            text: prompt,
            created_at,
        });
    }
    Ok(out)
}
