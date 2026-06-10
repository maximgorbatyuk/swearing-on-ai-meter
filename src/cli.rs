//! Command-line surface (clap derive).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "soaim",
    about = "Swearing on AI Meter — count swear words across your local AI-coding prompt history.",
    version
)]
pub struct Cli {
    /// Use an alternate config file.
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,

    /// Use an alternate database file.
    #[arg(long, value_name = "PATH", global = true)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Ingest new prompts without opening the TUI (cron-friendly).
    Ingest {
        /// Drop all caches and re-analyze every prompt.
        #[arg(long)]
        reset: bool,
    },
    /// Print today's swear count to stdout (no TUI).
    Today,
    /// Print a window's aggregates as text (no TUI).
    Stats {
        /// Window size in days.
        #[arg(long, default_value_t = 30)]
        days: u32,
    },
    /// Show resolved source paths + database path.
    Path,
    /// Inspect candidate source locations and report what is found.
    Discover,
}
