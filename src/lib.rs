//! soaim — Swearing on AI Meter.
//!
//! Library crate exposing every module so integration tests can exercise the
//! detector, sources, database, ingest pipeline, and stats directly. The
//! `soaim` binary is a thin shim over [`run`].

pub mod cli;
pub mod config;
pub mod db;
pub mod detector;
pub mod ingest;
pub mod model;
pub mod sources;
pub mod stats;
pub mod tui;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::config::{Config, Overrides};
use crate::db::Db;
use crate::model::Source;
use crate::stats::{AppFilter, Window};

/// Parse args and dispatch. Entry point for the binary.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let overrides = Overrides {
        config_path: cli.config.clone(),
        db_path: cli.db.clone(),
    };

    // Create a default config on first run (best-effort; never fatal).
    if overrides.config_path.is_none() {
        if let Ok(path) = config::default_config_path() {
            let _ = Config::write_default_if_missing(&path);
        }
    }

    let cfg = Config::load(&overrides)?;

    match cli.command {
        Some(Command::Path) => cmd_path(&cfg),
        Some(Command::Discover) => sources::discover::run(&cfg),
        Some(Command::Ingest { reset }) => cmd_ingest(&cfg, reset),
        Some(Command::Today) => cmd_today(&cfg),
        Some(Command::Stats { days }) => cmd_stats(&cfg, days),
        None => cmd_default(&cfg),
    }
}

fn cmd_path(cfg: &Config) -> Result<()> {
    println!("Resolved paths:");
    for src in Source::ALL {
        match cfg.paths.for_source(src) {
            Some(p) => {
                let mark = if p.exists() { "OK" } else { "--" };
                println!("  {mark} {:<14} {}", src.id(), p.display());
            }
            None => println!("  -- {:<14} (not located)", src.id()),
        }
    }
    println!("\nDatabase:\n  {}", cfg.db_path.display());
    println!("Swear words: {}", cfg.swears.len());
    Ok(())
}

fn cmd_ingest(cfg: &Config, reset: bool) -> Result<()> {
    let mut db = Db::open(&cfg.db_path)?;
    let report = ingest::run(cfg, &mut db, reset)?;
    println!(
        "Ingest complete. New prompts analyzed: {}",
        report.total_new()
    );
    for src in Source::ALL {
        let i = src.index();
        println!(
            "  {:<14} +{} new (of {} read), +{} swears",
            src.id(),
            report.new_per_source[i],
            report.read_per_source[i],
            report.swears_per_source[i]
        );
    }
    Ok(())
}

fn cmd_today(cfg: &Config) -> Result<()> {
    let mut db = Db::open(&cfg.db_path)?;
    eprintln!("Scanning prompt history… (first run may take a while)");
    ingest::run(cfg, &mut db, false)?;
    let dash = stats::dashboard(&db.conn, Window::D30, AppFilter::All)?;
    let t = &dash.today;
    println!("TODAY: {}", t.total());
    let breakdown: Vec<String> = Source::ALL
        .iter()
        .map(|s| format!("{} {}", s.short(), t.per_source[s.index()]))
        .collect();
    println!("  ({})", breakdown.join(" · "));
    Ok(())
}

fn cmd_stats(cfg: &Config, days: u32) -> Result<()> {
    let window = match days {
        0..=30 => Window::D30,
        31..=90 => Window::D90,
        _ => Window::D360,
    };
    let mut db = Db::open(&cfg.db_path)?;
    eprintln!("Scanning prompt history… (first run may take a while)");
    ingest::run(cfg, &mut db, false)?;
    let dash = stats::dashboard(&db.conn, window, AppFilter::All)?;

    println!("Window: last {} days\n", window.days());
    println!("Total swears: {}", dash.window_total.iter().sum::<u32>());
    for src in Source::ALL {
        println!("  {:<14} {}", src.id(), dash.window_total[src.index()]);
    }

    println!("\nPer weekday (Mon..Sun):");
    let labels = stats::weekday_labels();
    for (i, label) in labels.iter().enumerate() {
        println!("  {label} {}", dash.weekday[i].iter().sum::<u32>());
    }

    println!("\nPer hour (0..23):");
    for (h, slot) in dash.hourly.iter().enumerate() {
        let c: u32 = slot.iter().sum();
        if c > 0 {
            println!("  {h:02} {c}");
        }
    }
    Ok(())
}

fn cmd_default(cfg: &Config) -> Result<()> {
    let mut db = Db::open(&cfg.db_path)?;
    eprintln!("Scanning prompt history… (first run may take a while)");
    ingest::run(cfg, &mut db, false)?;
    tui::run(cfg, db)
}
