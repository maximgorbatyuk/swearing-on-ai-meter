//! Dashboard layout and rendering: the SOAIM banner, the activity heatmap, the
//! per-agent (last 14 days) comparison, and the filter + menu footer.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use super::app::App;
use super::widgets::{agents, heatmap, source_color};
use crate::model::Source;
use crate::stats::AppFilter;

/// Figlet-style "SOAIM" banner, rendered left-aligned in `render_banner`.
const LOGO: &str = r#" ____   ___    _    ___ __  __
/ ___| / _ \  / \  |_ _|  \/  |
\___ \| | | |/ _ \  | || |\/| |
 ___) | |_| / ___ \ | || |  | |
|____/ \___/_/   \_\___|_|  |_|"#;

const SUBTITLE: &str = "Know how many times you say *** to your agent";
const REPO: &str = "https://github.com/maximgorbatyuk/swearing-on-ai-meter";
const COPYRIGHT: &str = "(c) maximgorbatyuk";
/// Crate version, baked in at compile time (e.g. "v0.1.0").
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(9),  // SOAIM logo + subtitle + repo link + bottom spacing
        Constraint::Length(12), // middle row: heatmap | per-agent comparison
        Constraint::Length(3),  // filter + menu, a blank line, then the repo link
        Constraint::Min(0),     // spacer fills the rest below
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);

    // Heatmap flexes to fill the width (it is the genuinely-wide element); the
    // per-agent panel is fixed so it never sprawls on wide terminals.
    let middle = Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(chunks[1]);
    render_heatmap(frame, middle[0], app);
    render_agents(frame, middle[1], app);

    render_footer(frame, chunks[2], app);
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let accent = Style::new().fg(Color::Rgb(217, 119, 87)).bold();
    let subtitle_style = Style::new().fg(Color::Rgb(150, 150, 150)).italic();
    let dim_style = Style::new().fg(Color::Rgb(150, 150, 150));

    let mut lines: Vec<Line> = LOGO
        .lines()
        .map(|l| Line::from(Span::styled(l, accent)))
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(SUBTITLE, subtitle_style)));
    lines.push(Line::from(Span::styled(
        format!("{COPYRIGHT}  {VERSION}"),
        dim_style,
    )));
    lines.push(Line::from("")); // bottom spacing before the panels

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_heatmap(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" Last {} days (per day) ", app.window.days());
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    heatmap::render(frame.buffer_mut(), inner, &app.dash);
}

fn render_agents(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered().title(" Last 14 days · per agent ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    agents::render(frame.buffer_mut(), inner, &app.dash);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Borderless: the filter tokens (most used) + window/action keys on the
    // first line, the repo link on the second. The menu line clips on terminals
    // narrower than it; that is the accepted "slim" trade-off — every
    // keybinding still works regardless.
    let mut spans: Vec<Span> = vec![Span::raw("apps: ")];
    spans.push(filter_token(
        "[a]ll",
        app.filter == AppFilter::All,
        Color::White,
    ));
    spans.push(Span::raw(" "));
    let apps = [
        ("[1]claude", Source::ClaudeCode),
        ("[2]desktop", Source::ClaudeDesktop),
        ("[3]codex", Source::CodexCli),
        ("[4]codex-app", Source::CodexApp),
        ("[5]opencode", Source::Opencode),
    ];
    for (label, src) in apps {
        let active = app.filter == AppFilter::One(src);
        spans.push(filter_token(label, active, source_color(src)));
        spans.push(Span::raw(" "));
    }

    let key = |k: &str| Span::styled(k.to_string(), Style::new().bold());
    spans.push(Span::raw("  "));
    spans.push(key("[d]"));
    spans.push(Span::raw("30 "));
    spans.push(key("[9]"));
    spans.push(Span::raw("90 "));
    spans.push(key("[0]"));
    spans.push(Span::raw("360  "));
    spans.push(key("[r]"));
    spans.push(Span::raw("refresh "));
    spans.push(key("[s]"));
    spans.push(Span::raw("settings "));
    spans.push(key("[q]"));
    spans.push(Span::raw("quit"));

    let repo_style = Style::new().fg(Color::Rgb(110, 168, 254)).underlined();
    let repo = Line::from(Span::styled(REPO, repo_style));

    frame.render_widget(
        Paragraph::new(vec![Line::from(spans), Line::from(""), repo]),
        area,
    );
}

fn filter_token(label: &str, active: bool, color: Color) -> Span<'_> {
    let mut style = Style::new().fg(color);
    if active {
        style = style.bold().underlined();
    }
    Span::styled(label.to_string(), style)
}
