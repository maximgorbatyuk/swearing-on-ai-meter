//! Shared test helpers.

use std::path::{Path, PathBuf};

use soaim::config::{Config, SourcePaths};

/// Build a Config with every source pointed at a non-existent path, then let
/// callers override the ones they exercise.
pub fn empty_config(db_path: PathBuf) -> Config {
    Config {
        swears: vec!["fuck".to_string(), "shit".to_string(), "damn".to_string()],
        paths: SourcePaths {
            claude_code: PathBuf::from("/nonexistent/claude"),
            claude_desktop: None,
            codex_cli: PathBuf::from("/nonexistent/codex"),
            codex_app: None,
            opencode: PathBuf::from("/nonexistent/opencode.db"),
        },
        db_path,
    }
}

/// Write a file, creating parent directories.
pub fn write_file(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}
