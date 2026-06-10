//! The ingest pipeline: read every source, dedupe against the DB, detect
//! swears in new prompts only, store them, and rebuild the `hour_stat` cache —
//! all in one transaction. Idempotent: re-running ingests nothing new.
//!
//! Reads are also **incremental**: each source carries a per-source watermark
//! (the start time of its last successful ingest) and skips files unchanged
//! since, so steady-state runs re-parse almost nothing. The watermark is a
//! performance hint only — id-dedupe still guarantees correctness — so it is
//! captured *before* reading and advanced only after the run commits, ensuring
//! a file modified mid-run is re-read next time rather than missed.

use anyhow::Result;
use chrono::Utc;

use crate::config::Config;
use crate::db::Db;
use crate::detector::Detector;
use crate::model::Source;
use crate::sources::registry;

/// Outcome of one ingest run, for CLI reporting.
#[derive(Debug, Default, Clone)]
pub struct IngestReport {
    /// Prompts newly analyzed this run, per source.
    pub new_per_source: [u32; 5],
    /// Prompts read from disk this run, per source. With incremental reads
    /// this counts only prompts from files that changed since the watermark,
    /// not the whole history.
    pub read_per_source: [u32; 5],
    /// New swear occurrences recorded this run, per source.
    pub swears_per_source: [u32; 5],
}

impl IngestReport {
    pub fn total_new(&self) -> u32 {
        self.new_per_source.iter().sum()
    }
}

/// Run the full pipeline. `reset` drops all caches and re-analyzes everything.
pub fn run(cfg: &Config, db: &mut Db, reset: bool) -> Result<IngestReport> {
    if reset {
        db.reset()?;
    }

    let detector = Detector::new(cfg.swears.clone())?;
    let existing = db.existing_ids()?;
    // Captured before any reads: a file modified during this run has an mtime
    // >= this value and so is re-read next time (never silently skipped).
    let run_start = Utc::now().timestamp();
    let mut report = IngestReport::default();

    // Read all sources first (read-only on their files), then write in one tx.
    // Only sources that read without error advance their watermark.
    let mut to_insert: Vec<(String, Source, String, Option<i64>, u32)> = Vec::new();
    let mut read_ok = [false; 5];
    for src in registry() {
        let name = src.name();
        let since = db.get_watermark(name.id())?;
        let prompts = match src.read(cfg, since) {
            Ok(prompts) => {
                read_ok[name.index()] = true;
                prompts
            }
            // A failed source must not abort the run or advance its watermark,
            // so it is retried in full next time.
            Err(_) => continue,
        };
        report.read_per_source[name.index()] += prompts.len() as u32;
        for p in prompts {
            let id = p.id();
            if existing.contains(&id) {
                continue;
            }
            let count = detector.swear_count(&p.text);
            report.new_per_source[name.index()] += 1;
            report.swears_per_source[name.index()] += count;
            to_insert.push((id, name, p.text, p.created_at.map(|d| d.timestamp()), count));
        }
    }

    let tx = db.conn.transaction()?;
    for (id, source, prompt, created_at, count) in &to_insert {
        Db::insert_prompt(&tx, id, source.id(), prompt, *created_at, run_start, *count)?;
    }
    tx.commit()?;

    // Rebuild the rollup cache, then advance watermarks for sources that read
    // successfully (only now that their prompts are durably committed).
    db.rebuild_hour_stat(run_start)?;
    for src in Source::ALL {
        if read_ok[src.index()] {
            db.set_watermark(src.id(), run_start)?;
        }
    }
    db.set_meta("last_ingest", &run_start.to_string())?;

    Ok(report)
}
