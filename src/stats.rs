//! Aggregation queries producing chart-ready structs. `analyzed_prompt` is
//! authoritative; all day/hour/weekday bucketing is done in **local time** via
//! SQLite's `localtime` modifier, so charts match the user's lived experience.
//!
//! Every series carries a per-source breakdown (`[u32; 5]`, indexed by
//! `Source::index`) so the TUI can render stacked per-app segments. An
//! `AppFilter` restricts results to a single app when active.

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use rusqlite::Connection;

use crate::model::Source;

/// Number of per-source slots (the five apps).
pub const N_SOURCES: usize = 5;

/// Selectable time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    D30,
    D90,
    D360,
}

impl Window {
    pub fn days(self) -> i64 {
        match self {
            Window::D30 => 30,
            Window::D90 => 90,
            Window::D360 => 360,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Window::D30 => "30d",
            Window::D90 => "90d",
            Window::D360 => "360d",
        }
    }
}

/// Active per-app filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFilter {
    All,
    One(Source),
}

impl AppFilter {
    pub fn label(self) -> String {
        match self {
            AppFilter::All => "all".to_string(),
            AppFilter::One(s) => s.short().to_string(),
        }
    }
    /// Whether a given source should be counted under this filter.
    fn includes(self, src: Source) -> bool {
        match self {
            AppFilter::All => true,
            AppFilter::One(s) => s == src,
        }
    }
}

/// A per-source row of counts. `total()` sums across the included sources.
pub type PerSource = [u32; N_SOURCES];

fn total(p: &PerSource) -> u32 {
    p.iter().sum()
}

/// Today's swear count, broken down per app.
#[derive(Debug, Clone, Default)]
pub struct TodayCount {
    pub per_source: PerSource,
}
impl TodayCount {
    pub fn total(&self) -> u32 {
        total(&self.per_source)
    }
}

/// One day in the heatmap series.
#[derive(Debug, Clone)]
pub struct DayCell {
    pub date: NaiveDate,
    pub per_source: PerSource,
}
impl DayCell {
    pub fn total(&self) -> u32 {
        total(&self.per_source)
    }
}

/// The full set of chart data for the dashboard, for one (window, filter).
#[derive(Debug, Clone)]
pub struct Dashboard {
    pub window: Window,
    pub filter: AppFilter,
    pub today: TodayCount,
    pub daily: Vec<DayCell>,
    pub hourly: [PerSource; 24],
    pub weekday: [PerSource; 7], // Mon..Sun
    /// Grand total over the window, per source.
    pub window_total: PerSource,
    /// Per-app swear totals over the last 14 days, computed regardless of the
    /// active filter — drives the per-agent comparison chart.
    pub agents_14d: PerSource,
}

impl Dashboard {
    /// The max single-day total in the window (for heatmap intensity scaling).
    pub fn max_day(&self) -> u32 {
        self.daily.iter().map(|d| d.total()).max().unwrap_or(0)
    }
    pub fn max_hour(&self) -> u32 {
        self.hourly.iter().map(total).max().unwrap_or(0)
    }
    pub fn max_weekday(&self) -> u32 {
        self.weekday.iter().map(total).max().unwrap_or(0)
    }
}

/// Compute everything the dashboard needs in a handful of grouped queries.
pub fn dashboard(conn: &Connection, window: Window, filter: AppFilter) -> Result<Dashboard> {
    let today_local = Local::now().date_naive();
    let start = today_local - chrono::Duration::days(window.days() - 1);
    let start_str = start.format("%Y-%m-%d").to_string();
    let today_str = today_local.format("%Y-%m-%d").to_string();

    // --- today ---
    let mut today = TodayCount::default();
    for (src, c) in grouped_by_source(
        conn,
        "strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') = ?1",
        &today_str,
    )? {
        if filter.includes(src) {
            today.per_source[src.index()] = c;
        }
    }

    // --- daily series (zero-filled across the window) ---
    let mut by_day: std::collections::HashMap<(String, Source), u32> =
        std::collections::HashMap::new();
    {
        let sql = "SELECT strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') AS day, \
                          source, SUM(swear_count) \
                   FROM analyzed_prompt \
                   WHERE created_at IS NOT NULL \
                     AND strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') >= ?1 \
                   GROUP BY day, source";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([&start_str], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (day, source, count) = row?;
            if let Some(src) = Source::from_id(&source) {
                if filter.includes(src) {
                    by_day.insert((day, src), count as u32);
                }
            }
        }
    }
    let mut daily = Vec::with_capacity(window.days() as usize);
    let mut window_total: PerSource = [0; N_SOURCES];
    let mut d = start;
    while d <= today_local {
        let key = d.format("%Y-%m-%d").to_string();
        let mut per_source: PerSource = [0; N_SOURCES];
        for src in Source::ALL {
            if let Some(c) = by_day.get(&(key.clone(), src)) {
                per_source[src.index()] = *c;
                window_total[src.index()] += *c;
            }
        }
        daily.push(DayCell {
            date: d,
            per_source,
        });
        d += chrono::Duration::days(1);
    }

    // --- per hour ---
    let mut hourly: [PerSource; 24] = [[0; N_SOURCES]; 24];
    {
        let sql =
            "SELECT CAST(strftime('%H', created_at, 'unixepoch', 'localtime') AS INTEGER) AS hr, \
                          source, SUM(swear_count) \
                   FROM analyzed_prompt \
                   WHERE created_at IS NOT NULL \
                     AND strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') >= ?1 \
                   GROUP BY hr, source";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([&start_str], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (hr, source, count) = row?;
            if let (Some(src), 0..=23) = (Source::from_id(&source), hr) {
                if filter.includes(src) {
                    hourly[hr as usize][src.index()] = count as u32;
                }
            }
        }
    }

    // --- per weekday (SQLite %w: 0=Sun..6=Sat; remap to Mon..Sun) ---
    let mut weekday: [PerSource; 7] = [[0; N_SOURCES]; 7];
    {
        let sql =
            "SELECT CAST(strftime('%w', created_at, 'unixepoch', 'localtime') AS INTEGER) AS wd, \
                          source, SUM(swear_count) \
                   FROM analyzed_prompt \
                   WHERE created_at IS NOT NULL \
                     AND strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') >= ?1 \
                   GROUP BY wd, source";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([&start_str], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (wd, source, count) = row?;
            if let Some(src) = Source::from_id(&source) {
                if filter.includes(src) {
                    let idx = ((wd + 6) % 7) as usize; // Sun(0)->6, Mon(1)->0, ...
                    weekday[idx][src.index()] = count as u32;
                }
            }
        }
    }

    // --- per-agent totals over the last 14 days (for the comparison chart) ---
    // Intentionally NOT filtered: the chart always compares all five apps.
    let agents_start = (today_local - chrono::Duration::days(13))
        .format("%Y-%m-%d")
        .to_string();
    let mut agents_14d: PerSource = [0; N_SOURCES];
    for (src, c) in grouped_by_source(
        conn,
        "strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') >= ?1",
        &agents_start,
    )? {
        agents_14d[src.index()] = c;
    }

    Ok(Dashboard {
        window,
        filter,
        today,
        daily,
        hourly,
        weekday,
        window_total,
        agents_14d,
    })
}

/// Helper: run a `GROUP BY source` query with a single `?1` string parameter,
/// returning (Source, count) pairs.
fn grouped_by_source(
    conn: &Connection,
    where_clause: &str,
    param: &str,
) -> Result<Vec<(Source, u32)>> {
    let sql = format!(
        "SELECT source, SUM(swear_count) FROM analyzed_prompt \
         WHERE created_at IS NOT NULL AND {where_clause} GROUP BY source"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([param], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (source, count) = row?;
        if let Some(src) = Source::from_id(&source) {
            out.push((src, count as u32));
        }
    }
    Ok(out)
}

/// Plain-text totals for `soaim today` / `soaim stats`.
pub fn weekday_labels() -> [&'static str; 7] {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
}

/// Convenience used by the heatmap: the weekday index (Mon=0..Sun=6) of a date.
pub fn weekday_index(date: NaiveDate) -> usize {
    date.weekday().num_days_from_monday() as usize
}
