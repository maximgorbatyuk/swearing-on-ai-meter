//! soaim's own SQLite database: schema, migrations, dedupe inserts, and the
//! `hour_stat` rollup cache. `analyzed_prompt` is authoritative; `hour_stat`
//! is an optional fast path keyed by (day, hour) in local time.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

/// Bump when the schema changes.
const SCHEMA_VERSION: i64 = 1;

pub struct Db {
    pub conn: Connection,
}

impl Db {
    /// Open (creating the file + parent dir if needed) and run migrations.
    pub fn open(path: &Path) -> Result<Db> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    /// In-memory database, for tests.
    pub fn open_in_memory() -> Result<Db> {
        let conn = Connection::open_in_memory()?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )?;
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS analyzed_prompt (
                    id           TEXT PRIMARY KEY,
                    source       TEXT NOT NULL,
                    prompt       TEXT NOT NULL,
                    created_at   INTEGER,            -- unix seconds (UTC); NULL if unknown
                    processed_at INTEGER NOT NULL,
                    swear_count  INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_ap_created ON analyzed_prompt(created_at);
                CREATE INDEX IF NOT EXISTS idx_ap_source  ON analyzed_prompt(source);

                CREATE TABLE IF NOT EXISTS hour_stat (
                    source       TEXT NOT NULL,
                    day          TEXT NOT NULL,      -- 'YYYY-MM-DD' (local)
                    hour         INTEGER NOT NULL,   -- 0..23 (local)
                    swear_count  INTEGER NOT NULL,
                    created_at   INTEGER NOT NULL,
                    PRIMARY KEY (source, day, hour)
                );

                CREATE TABLE IF NOT EXISTS meta (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                "#,
            )?;
        }
        self.conn
            .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        Ok(())
    }

    /// Set of ids already present, so ingest only analyzes new prompts.
    pub fn existing_ids(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT id FROM analyzed_prompt")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut set = std::collections::HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    /// Insert an analyzed prompt, ignoring duplicates by primary key.
    pub fn insert_prompt(
        tx: &Connection,
        id: &str,
        source: &str,
        prompt: &str,
        created_at: Option<i64>,
        processed_at: i64,
        swear_count: u32,
    ) -> Result<()> {
        tx.execute(
            "INSERT OR IGNORE INTO analyzed_prompt
                (id, source, prompt, created_at, processed_at, swear_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, source, prompt, created_at, processed_at, swear_count],
        )?;
        Ok(())
    }

    /// Rebuild the entire `hour_stat` cache from `analyzed_prompt`, bucketing in
    /// local time. Cheap relative to a full re-scan and keeps the cache exact.
    pub fn rebuild_hour_stat(&self, now: i64) -> Result<()> {
        self.conn.execute("DELETE FROM hour_stat", [])?;
        self.conn.execute(
            "INSERT INTO hour_stat (source, day, hour, swear_count, created_at)
             SELECT source,
                    strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') AS day,
                    CAST(strftime('%H', created_at, 'unixepoch', 'localtime') AS INTEGER) AS hour,
                    SUM(swear_count),
                    ?1
             FROM analyzed_prompt
             WHERE created_at IS NOT NULL
             GROUP BY source, day, hour",
            params![now],
        )?;
        Ok(())
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let v = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get::<_, String>(0)
            })
            .ok();
        Ok(v)
    }

    /// Per-source incremental-read watermark (unix seconds): the start time of
    /// the last successful ingest of that source. `read` skips files unmodified
    /// since. `None` until the first successful ingest of the source.
    pub fn get_watermark(&self, source: &str) -> Result<Option<i64>> {
        Ok(self
            .get_meta(&format!("read_watermark:{source}"))?
            .and_then(|s| s.parse::<i64>().ok()))
    }

    pub fn set_watermark(&self, source: &str, ts: i64) -> Result<()> {
        self.set_meta(&format!("read_watermark:{source}"), &ts.to_string())
    }

    /// Drop all cached analysis (for `ingest --reset`).
    pub fn reset(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM analyzed_prompt; DELETE FROM hour_stat; DELETE FROM meta;",
        )?;
        Ok(())
    }
}
