//! Terminal UI: ingest happens before `run` is called; this owns the terminal,
//! the app state, and the event loop. Terminal raw mode + alternate screen are
//! entered via `ratatui::try_init` and always restored on exit (and on panic,
//! via ratatui's installed panic hook).

pub mod app;
pub mod ui;
pub mod widgets;

use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::config::Config;
use crate::db::Db;
use crate::ingest;
use crate::model::Source;
use crate::stats::{AppFilter, Window};

use self::app::App;

pub fn run(cfg: &Config, mut db: Db) -> Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = run_loop(cfg, &mut db, &mut terminal);
    ratatui::restore();
    result
}

fn run_loop(cfg: &Config, db: &mut Db, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new(&db.conn, Window::D30, AppFilter::All)?;

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Char('c') if ctrl => break,
            KeyCode::Char('r') => {
                ingest::run(cfg, db, false)?;
                app.refresh(&db.conn)?;
            }
            // Open the config file in the OS's default text editor. Failures are
            // ignored so a missing launcher never tears down the TUI.
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let _ = open_settings(cfg);
            }
            KeyCode::Char('9') => app.set_window(&db.conn, Window::D90)?,
            KeyCode::Char('0') => app.set_window(&db.conn, Window::D360)?,
            KeyCode::Char('d') | KeyCode::Esc => app.set_window(&db.conn, Window::D30)?,
            KeyCode::Char('a') => app.set_filter(&db.conn, AppFilter::All)?,
            KeyCode::Char('1') => app.set_filter(&db.conn, AppFilter::One(Source::ClaudeCode))?,
            KeyCode::Char('2') => {
                app.set_filter(&db.conn, AppFilter::One(Source::ClaudeDesktop))?
            }
            KeyCode::Char('3') => app.set_filter(&db.conn, AppFilter::One(Source::CodexCli))?,
            KeyCode::Char('4') => app.set_filter(&db.conn, AppFilter::One(Source::CodexApp))?,
            KeyCode::Char('5') => app.set_filter(&db.conn, AppFilter::One(Source::Opencode))?,
            _ => {}
        }
    }
    Ok(())
}

/// Open the config file in the OS's default GUI text editor, creating a default
/// one first if it doesn't exist yet. A GUI app (not `$EDITOR`) is used on
/// purpose: a terminal editor would fight the TUI's alternate screen. The
/// launcher is spawned detached so the dashboard keeps running.
fn open_settings(cfg: &Config) -> Result<()> {
    let path = &cfg.config_path;
    let _ = Config::write_default_if_missing(path);

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg("-t").arg(path); // -t => default text-editor app
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(path);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };

    cmd.spawn()
        .with_context(|| format!("opening {} in a text editor", path.display()))?;
    Ok(())
}
