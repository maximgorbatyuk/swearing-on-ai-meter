//! Codex App (desktop/IDE).
//!
//! Storage is **not yet fully reverse-engineered** (run `soaim discover`).
//! Until the prompt store is located, this source does a best-effort scan of
//! the configured directory for JSONL chat logs using the same record rules as
//! the Codex CLI — covering the case where the app mirrors that format — and
//! otherwise yields zero prompts so the other sources keep working.

use anyhow::Result;
use serde_json::Value;
use walkdir::WalkDir;

use super::{fallback_id, file_changed_since, parse_ts, text_from_content, PromptSource};
use crate::config::Config;
use crate::model::{RawPrompt, Source};

pub struct CodexApp;

impl PromptSource for CodexApp {
    fn name(&self) -> Source {
        Source::CodexApp
    }

    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>> {
        let Some(root) = cfg.paths.codex_app.as_ref() else {
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

                // Flat history-style entry.
                if let Some(flat) = obj
                    .get("text")
                    .or_else(|| obj.get("prompt"))
                    .or_else(|| obj.get("input"))
                    .and_then(Value::as_str)
                {
                    if obj.get("role").is_none() && !flat.trim().is_empty() {
                        let ts = obj.get("ts").or_else(|| obj.get("timestamp"));
                        out.push(RawPrompt {
                            source: Source::CodexApp,
                            native_id: fallback_id(&format!(
                                "{}|{}|{}",
                                path.to_string_lossy(),
                                lineno,
                                flat
                            )),
                            text: flat.to_string(),
                            created_at: ts.and_then(parse_ts),
                        });
                        continue;
                    }
                }

                let (payload, role) = match obj.get("role").and_then(Value::as_str) {
                    Some(r) => (&obj, Some(r)),
                    None => match obj.get("message") {
                        Some(m) => (m, m.get("role").and_then(Value::as_str)),
                        None => (&obj, None),
                    },
                };
                if role != Some("user") {
                    continue;
                }
                let Some(content) = payload.get("content") else {
                    continue;
                };
                let prompt = text_from_content(content);
                if prompt.trim().is_empty() {
                    continue;
                }
                let ts = obj.get("timestamp").or_else(|| obj.get("ts"));
                let native_id = payload
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        fallback_id(&format!("{}|{}|{}", path.to_string_lossy(), lineno, prompt))
                    });
                out.push(RawPrompt {
                    source: Source::CodexApp,
                    native_id,
                    text: prompt,
                    created_at: ts.and_then(parse_ts),
                });
            }
        }
        Ok(out)
    }
}
