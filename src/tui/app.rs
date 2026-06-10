//! TUI application state. Holds the current window + filter and the cached
//! `Dashboard`, recomputing only when those change or on refresh.

use anyhow::Result;
use rusqlite::Connection;

use crate::stats::{self, AppFilter, Dashboard, Window};

pub struct App {
    pub window: Window,
    pub filter: AppFilter,
    pub dash: Dashboard,
}

impl App {
    pub fn new(conn: &Connection, window: Window, filter: AppFilter) -> Result<App> {
        let dash = stats::dashboard(conn, window, filter)?;
        Ok(App {
            window,
            filter,
            dash,
        })
    }

    /// Recompute the dashboard for the current window + filter.
    pub fn refresh(&mut self, conn: &Connection) -> Result<()> {
        self.dash = stats::dashboard(conn, self.window, self.filter)?;
        Ok(())
    }

    pub fn set_window(&mut self, conn: &Connection, window: Window) -> Result<()> {
        if self.window != window {
            self.window = window;
            self.refresh(conn)?;
        }
        Ok(())
    }

    pub fn set_filter(&mut self, conn: &Connection, filter: AppFilter) -> Result<()> {
        if self.filter != filter {
            self.filter = filter;
            self.refresh(conn)?;
        }
        Ok(())
    }
}
