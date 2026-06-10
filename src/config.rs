//! Configuration: load + merge TOML config with built-in defaults, and resolve
//! all on-disk paths. Precedence: CLI flags > config file > built-in defaults.

use anyhow::{Context, Result};
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::detector::default_words;
use crate::model::Source;

/// Fully-resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Swear word list (whole-word, case-insensitive).
    pub swears: Vec<String>,
    /// Resolved root path per source (where its history lives).
    pub paths: SourcePaths,
    /// Resolved path to soaim's own SQLite database.
    pub db_path: PathBuf,
}

/// One resolved root per source.
#[derive(Debug, Clone)]
pub struct SourcePaths {
    pub claude_code: PathBuf,
    pub claude_desktop: Option<PathBuf>,
    pub codex_cli: PathBuf,
    pub codex_app: Option<PathBuf>,
    pub opencode: PathBuf,
}

impl SourcePaths {
    /// The resolved root for a given source (if known).
    pub fn for_source(&self, src: Source) -> Option<&Path> {
        match src {
            Source::ClaudeCode => Some(&self.claude_code),
            Source::ClaudeDesktop => self.claude_desktop.as_deref(),
            Source::CodexCli => Some(&self.codex_cli),
            Source::CodexApp => self.codex_app.as_deref(),
            Source::Opencode => Some(&self.opencode),
        }
    }
}

/// The on-disk TOML shape. Every field is optional so a partial file is valid.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FileConfig {
    #[serde(default)]
    pub swears: Option<Vec<String>>,
    #[serde(default)]
    pub paths: FilePaths,
    #[serde(default)]
    pub db: FileDb,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FilePaths {
    pub claude: Option<String>,
    pub claude_desktop: Option<String>,
    pub codex: Option<String>,
    pub codex_app: Option<String>,
    pub opencode: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FileDb {
    pub path: Option<String>,
}

/// CLI-level overrides (from flags). All optional.
#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub config_path: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
}

impl Config {
    /// Load config, applying defaults then the config file then CLI overrides.
    pub fn load(overrides: &Overrides) -> Result<Config> {
        let config_path = match &overrides.config_path {
            Some(p) => p.clone(),
            None => default_config_path()?,
        };

        let file = read_file_config(&config_path)?;

        let swears = file
            .swears
            .filter(|v| !v.is_empty())
            .unwrap_or_else(default_words);

        let home = BaseDirs::new()
            .map(|b| b.home_dir().to_path_buf())
            .context("could not determine home directory")?;

        let paths = SourcePaths {
            claude_code: resolve(file.paths.claude, || home.join(".claude/projects")),
            // Desktop apps default to disabled: their on-disk prompt store
            // hasn't been located, and auto-probing their multi-GB Electron
            // data dirs makes every run crawl. They resolve only when the user
            // sets an explicit path in config; `soaim discover` still probes the
            // candidate locations (see `candidate_dirs`).
            claude_desktop: file.paths.claude_desktop.map(|p| expanduser(&p, &home)),
            codex_cli: resolve(file.paths.codex, || home.join(".codex")),
            codex_app: file.paths.codex_app.map(|p| expanduser(&p, &home)),
            opencode: resolve(file.paths.opencode, || default_opencode_db(&home)),
        };

        let db_path = overrides
            .db_path
            .clone()
            .or_else(|| file.db.path.map(|p| expanduser(&p, &home)))
            .map(Ok)
            .unwrap_or_else(default_db_path)?;

        Ok(Config {
            swears,
            paths,
            db_path,
        })
    }

    /// Write a default config file to `path` if none exists. Returns whether a
    /// file was created.
    pub fn write_default_if_missing(path: &Path) -> Result<bool> {
        if path.exists() {
            return Ok(false);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
        }
        std::fs::write(path, DEFAULT_CONFIG_TOML)
            .with_context(|| format!("writing default config {}", path.display()))?;
        Ok(true)
    }
}

fn resolve(opt: Option<String>, default: impl FnOnce() -> PathBuf) -> PathBuf {
    match opt {
        Some(p) => {
            let home = BaseDirs::new().map(|b| b.home_dir().to_path_buf());
            match home {
                Some(h) => expanduser(&p, &h),
                None => PathBuf::from(p),
            }
        }
        None => default(),
    }
}

fn read_file_config(path: &Path) -> Result<FileConfig> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    let cfg: FileConfig =
        toml::from_str(&text).with_context(|| format!("parsing config {}", path.display()))?;
    Ok(cfg)
}

/// Expand a leading `~` to the home directory.
fn expanduser(p: &str, home: &Path) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        home.join(rest)
    } else if p == "~" {
        home.to_path_buf()
    } else {
        PathBuf::from(p)
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    if let Some(dirs) = ProjectDirs::from("", "", "soaim") {
        return Ok(dirs.config_dir().join("config.toml"));
    }
    let home = BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .context("could not determine home directory")?;
    Ok(home.join(".config/soaim/config.toml"))
}

pub fn default_db_path() -> Result<PathBuf> {
    if let Some(dirs) = ProjectDirs::from("", "", "soaim") {
        return Ok(dirs.data_dir().join("soaim.db"));
    }
    let home = BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .context("could not determine home directory")?;
    Ok(home.join(".local/share/soaim/soaim.db"))
}

fn default_opencode_db(home: &Path) -> PathBuf {
    // Mirrors the PoC's search order; first existing wins, else the XDG default.
    let candidates = [
        std::env::var_os("XDG_DATA_HOME").map(|x| PathBuf::from(x).join("opencode/opencode.db")),
        Some(home.join(".local/share/opencode/opencode.db")),
        Some(home.join("Library/Application Support/opencode/opencode.db")),
        Some(home.join(".config/opencode/opencode.db")),
        Some(home.join(".opencode/opencode.db")),
    ];
    for c in candidates.into_iter().flatten() {
        if c.exists() {
            return c;
        }
    }
    home.join(".local/share/opencode/opencode.db")
}

/// Candidate directories to probe for a source whose prompt store isn't
/// configured. Used only by `soaim discover` to help locate the desktop apps'
/// stores — normal ingest never touches these (the desktop sources stay
/// disabled until an explicit path is configured). Returns empty for the
/// already-located sources.
pub fn candidate_dirs(src: Source) -> Vec<PathBuf> {
    let Some(home) = BaseDirs::new().map(|b| b.home_dir().to_path_buf()) else {
        return Vec::new();
    };
    match src {
        Source::ClaudeDesktop => vec![
            home.join("Library/Application Support/Claude"),
            home.join(".config/Claude"),
            home.join(".config/claude-desktop"),
        ],
        Source::CodexApp => vec![
            home.join("Library/Application Support/Codex"),
            home.join("Library/Application Support/ChatGPT"),
            home.join(".config/Codex"),
        ],
        _ => Vec::new(),
    }
}

const DEFAULT_CONFIG_TOML: &str = r#"# soaim configuration
# Words are matched whole-word and case-insensitively.
# Uncomment and edit to override the built-in list.
# swears = ["fuck", "shit", "damn"]

[paths]
# Override any source's location. "~" is expanded to your home directory.
# claude         = "~/.claude/projects"
# claude_desktop = "~/Library/Application Support/Claude"
# codex          = "~/.codex"
# codex_app      = "~/Library/Application Support/Codex"
# opencode       = "~/.local/share/opencode/opencode.db"

[db]
# path = "~/.local/share/soaim/soaim.db"
"#;
