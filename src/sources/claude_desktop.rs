//! Claude Desktop app.
//!
//! Storage is **not yet fully reverse-engineered**: on macOS the desktop app
//! keeps chat state largely in an IndexedDB/LevelDB store under
//! `~/Library/Application Support/Claude/`, which is not a stable, documented
//! prompt log. Until the prompt store is located (run `soaim discover`), this
//! source does a best-effort scan for JSONL chat logs in the configured
//! directory using the same record rules as Claude Code — covering the case
//! where the desktop app mirrors that format — and otherwise yields zero
//! prompts so the other sources keep working.

use anyhow::Result;
use serde_json::Value;
use walkdir::WalkDir;

use super::{
    fallback_id, file_changed_since, is_all_tool_result, parse_ts, text_from_content, PromptSource,
};
use crate::config::Config;
use crate::model::{RawPrompt, Source};

pub struct ClaudeDesktop;

impl PromptSource for ClaudeDesktop {
    fn name(&self) -> Source {
        Source::ClaudeDesktop
    }

    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>> {
        let Some(root) = cfg.paths.claude_desktop.as_ref() else {
            return Ok(Vec::new());
        };
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
        {
            let path = entry.path();
            if !file_changed_since(path, since) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            for (lineno, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Ok(obj) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if obj.get("type").and_then(Value::as_str) != Some("user") {
                    continue;
                }
                let Some(msg) = obj.get("message") else {
                    continue;
                };
                if msg.get("role").and_then(Value::as_str) != Some("user") {
                    continue;
                }
                let Some(content) = msg.get("content") else {
                    continue;
                };
                if is_all_tool_result(content) {
                    continue;
                }
                let prompt = text_from_content(content);
                if prompt.trim().is_empty() {
                    continue;
                }
                let native_id = obj
                    .get("uuid")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("messageId").and_then(Value::as_str))
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        fallback_id(&format!("{}|{}", path.to_string_lossy(), lineno))
                    });
                let created_at = obj.get("timestamp").and_then(parse_ts);
                out.push(RawPrompt {
                    source: Source::ClaudeDesktop,
                    native_id,
                    text: prompt,
                    created_at,
                });
            }
        }
        Ok(out)
    }
}
