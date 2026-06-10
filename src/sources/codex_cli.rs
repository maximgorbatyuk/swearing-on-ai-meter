//! Codex CLI: `~/.codex/history.jsonl` plus rollout sessions
//! `~/.codex/**/*.jsonl`.
//!
//! `history.jsonl` holds flat entries with a `text` / `prompt` / `input`
//! field. Rollout session files hold chat records; user prompts are records
//! with `role == "user"` (possibly nested under `message`).

use anyhow::Result;
use serde_json::Value;
use walkdir::WalkDir;

use super::{fallback_id, file_changed_since, parse_ts, text_from_content, PromptSource};
use crate::config::Config;
use crate::model::{RawPrompt, Source};

pub struct CodexCli;

impl PromptSource for CodexCli {
    fn name(&self) -> Source {
        Source::CodexCli
    }

    fn read(&self, cfg: &Config, since: Option<i64>) -> Result<Vec<RawPrompt>> {
        let root = &cfg.paths.codex_cli;
        let mut out = Vec::new();
        if !root.exists() {
            return Ok(out);
        }

        let history = root.join("history.jsonl");

        // 1) Flat history file (re-read whole when it has grown; dedupe drops
        //    the lines we've already stored).
        if history.exists() && file_changed_since(&history, since) {
            if let Ok(text) = std::fs::read_to_string(&history) {
                for (lineno, line) in text.lines().enumerate() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let Ok(obj) = serde_json::from_str::<Value>(line) else {
                        continue;
                    };
                    let prompt = obj
                        .get("text")
                        .or_else(|| obj.get("prompt"))
                        .or_else(|| obj.get("input"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if prompt.trim().is_empty() {
                        continue;
                    }
                    let ts = obj.get("ts").or_else(|| obj.get("timestamp"));
                    let created_at = ts.and_then(parse_ts);
                    let native_id = obj
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            fallback_id(&format!(
                                "history|{}|{}|{}",
                                lineno,
                                prompt,
                                ts.map(|v| v.to_string()).unwrap_or_default()
                            ))
                        });
                    out.push(RawPrompt {
                        source: Source::CodexCli,
                        native_id,
                        text: prompt,
                        created_at,
                    });
                }
            }
        }

        // 2) Rollout session files (every other *.jsonl under the root).
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
            .filter(|e| e.path() != history)
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

                // role may be top-level or nested under `message`.
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
                let created_at = ts.and_then(parse_ts);
                let native_id = payload
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        fallback_id(&format!("{}|{}|{}", path.to_string_lossy(), lineno, prompt))
                    });

                out.push(RawPrompt {
                    source: Source::CodexCli,
                    native_id,
                    text: prompt,
                    created_at,
                });
            }
        }

        Ok(out)
    }
}
