//! Claude Code: `~/.claude/projects/**/*.jsonl`.
//!
//! User prompts are JSONL records where `type == "user"` and
//! `message.role == "user"`. Turns whose content is entirely `tool_result`
//! blocks carry no user text and are skipped. `native_id` prefers the event's
//! `uuid`, falling back to a hash of (file, line).

use anyhow::Result;
use serde_json::Value;
use walkdir::WalkDir;

use super::{
    fallback_id, file_changed_since, is_all_tool_result, parse_ts, text_from_content, PromptSource,
};
use crate::config::Config;
use crate::model::{RawPrompt, Source};

pub struct ClaudeCode;

impl PromptSource for ClaudeCode {
    fn name(&self) -> Source {
        Source::ClaudeCode
    }

    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>> {
        let root = &cfg.paths.claude_code;
        let mut out = Vec::new();
        if !root.exists() {
            return Ok(out);
        }

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
                    .or_else(|| msg.get("id").and_then(Value::as_str))
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        fallback_id(&format!("{}|{}", path.to_string_lossy(), lineno))
                    });

                let created_at = obj.get("timestamp").and_then(parse_ts);

                out.push(RawPrompt {
                    source: Source::ClaudeCode,
                    native_id,
                    text: prompt,
                    created_at,
                });
            }
        }
        Ok(out)
    }
}
