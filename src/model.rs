//! Core data structures shared across the crate.

use chrono::{DateTime, Utc};
use std::fmt;

/// The five AI-coding apps whose prompt history `soaim` scans.
///
/// This enum is the fixed, ordered grouping key for every per-app statistic.
/// Apps with no data render as zero, never missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    ClaudeCode,
    ClaudeDesktop,
    CodexCli,
    CodexApp,
    Opencode,
}

impl Source {
    /// Every source, in canonical display order. The single source of truth
    /// for iteration so per-app stats are always complete and consistently
    /// ordered.
    pub const ALL: [Source; 5] = [
        Source::ClaudeCode,
        Source::ClaudeDesktop,
        Source::CodexCli,
        Source::CodexApp,
        Source::Opencode,
    ];

    /// Stable string id stored in the database (`analyzed_prompt.source`).
    pub fn id(self) -> &'static str {
        match self {
            Source::ClaudeCode => "claude",
            Source::ClaudeDesktop => "claude_desktop",
            Source::CodexCli => "codex",
            Source::CodexApp => "codex_app",
            Source::Opencode => "opencode",
        }
    }

    /// Short label used in the compact header breakdown.
    pub fn short(self) -> &'static str {
        match self {
            Source::ClaudeCode => "claude",
            Source::ClaudeDesktop => "desktop",
            Source::CodexCli => "codex",
            Source::CodexApp => "codex-app",
            Source::Opencode => "opencode",
        }
    }

    /// Human-friendly display name.
    pub fn display(self) -> &'static str {
        match self {
            Source::ClaudeCode => "Claude Code",
            Source::ClaudeDesktop => "Claude Desktop",
            Source::CodexCli => "Codex CLI",
            Source::CodexApp => "Codex App",
            Source::Opencode => "opencode",
        }
    }

    /// Parse the stable id back into a `Source`.
    pub fn from_id(s: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|src| src.id() == s)
    }

    /// Stable index 0..5 — used for fixed per-app colors and array slots.
    pub fn index(self) -> usize {
        match self {
            Source::ClaudeCode => 0,
            Source::ClaudeDesktop => 1,
            Source::CodexCli => 2,
            Source::CodexApp => 3,
            Source::Opencode => 4,
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A single user-authored prompt as read from a tool's history, before
/// analysis. `assistant` / tool output is never represented here.
#[derive(Debug, Clone)]
pub struct RawPrompt {
    /// Which of the five apps this came from.
    pub source: Source,
    /// Stable id from the tool, used for dedupe. Combined with `source` to
    /// form the primary key `"{source}:{native_id}"`.
    pub native_id: String,
    /// User-authored text only.
    pub text: String,
    /// Prompt timestamp. `None` if the tool stored no parseable timestamp.
    pub created_at: Option<DateTime<Utc>>,
}

impl RawPrompt {
    /// The composite primary key used in the database.
    pub fn id(&self) -> String {
        format!("{}:{}", self.source.id(), self.native_id)
    }
}
