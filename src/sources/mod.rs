//! Prompt sources: one implementation per AI-coding app, behind a common
//! trait. Adding another tool later is just another `PromptSource`.

pub mod claude_code;
pub mod claude_desktop;
pub mod codex_app;
pub mod codex_cli;
pub mod discover;
pub mod opencode;

use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use crate::config::Config;
use crate::model::{RawPrompt, Source};

/// A reader of user-authored prompts for one app.
pub trait PromptSource {
    /// Which of the five apps this reads.
    fn name(&self) -> Source;

    /// Yield only user-authored prompts, skipping assistant and tool output.
    /// A missing/uninstalled source must return `Ok(vec![])`, never an error,
    /// so the other sources keep working.
    ///
    /// `since` is an incremental-read watermark (unix seconds): when `Some`, a
    /// source may skip any file whose mtime predates it, since an unchanged
    /// file cannot hold prompts we haven't already stored. `None` reads
    /// everything. This is purely a performance hint — correctness never
    /// depends on it, because ingest dedupes by id — so a source may always
    /// read more than strictly necessary, and must when in doubt.
    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>>;
}

/// All five sources, in canonical order.
pub fn registry() -> Vec<Box<dyn PromptSource>> {
    vec![
        Box::new(claude_code::ClaudeCode),
        Box::new(claude_desktop::ClaudeDesktop),
        Box::new(codex_cli::CodexCli),
        Box::new(codex_app::CodexApp),
        Box::new(opencode::Opencode),
    ]
}

// ---------------------------------------------------------------------------
// Shared parsing helpers (ported from the validated Python PoC).
// ---------------------------------------------------------------------------

/// Parse a timestamp that may be epoch seconds, epoch milliseconds, or an
/// ISO-8601 string. Mirrors the PoC's `parse_ts`.
pub fn parse_ts(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::Number(n) => {
            let mut v = n.as_f64()?;
            if v > 1e12 {
                v /= 1000.0; // milliseconds -> seconds
            }
            let secs = v.trunc() as i64;
            let nanos = ((v - v.trunc()) * 1e9).round() as u32;
            Utc.timestamp_opt(secs, nanos).single()
        }
        Value::String(s) => parse_ts_str(s),
        _ => None,
    }
}

/// Parse an epoch-millisecond integer (opencode `time_created`).
pub fn parse_epoch_millis(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

fn parse_ts_str(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Try RFC3339 first (handles trailing Z and offsets).
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s.replace('Z', "+00:00")) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Bare datetime with no offset -> assume UTC.
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    None
}

/// Extract user text from a `content` field that may be a plain string or a
/// list of text blocks/parts. Mirrors the PoC's `text_from_content`.
pub fn text_from_content(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => {
            let mut parts: Vec<String> = Vec::new();
            for block in blocks {
                match block {
                    Value::String(s) => parts.push(s.clone()),
                    Value::Object(map) => {
                        let kind = map.get("type").and_then(Value::as_str);
                        let is_text = kind.is_none() || kind == Some("text");
                        if is_text {
                            if let Some(t) = map.get("text").and_then(Value::as_str) {
                                parts.push(t.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

/// True if a `content` array is made up entirely of `tool_result` blocks (a
/// turn that carries no user-authored text). Such turns are skipped.
pub fn is_all_tool_result(content: &Value) -> bool {
    match content {
        Value::Array(blocks) if !blocks.is_empty() => blocks.iter().all(|b| {
            b.as_object()
                .and_then(|m| m.get("type"))
                .and_then(Value::as_str)
                == Some("tool_result")
        }),
        _ => false,
    }
}

/// Whether a file should be read under the incremental watermark `since`
/// (unix seconds). Reads unconditionally when `since` is `None` or the file's
/// mtime can't be read; otherwise skips files last modified strictly before
/// `since`. Skipping is safe: any append/edit bumps the mtime, and id-dedupe is
/// the correctness backstop, so we only ever skip files we are sure are
/// unchanged and err toward re-reading on any ambiguity.
pub fn file_changed_since(path: &Path, since: Option<i64>) -> bool {
    let Some(since) = since else {
        return true;
    };
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => match mtime.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64 >= since,
            Err(_) => true, // mtime before the epoch; just read it
        },
        Err(_) => true, // can't stat; attempt the read
    }
}

/// Stable fallback id: an FNV-1a hash of `key`, hex-encoded. Used when a tool
/// stores no native id. Callers build `key` from stable parts (e.g.
/// `format!("{path}|{line}")`).
pub fn fallback_id(key: &str) -> String {
    // FNV-1a 64-bit — deterministic, dependency-free, good enough for dedupe.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
