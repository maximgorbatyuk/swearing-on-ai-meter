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

/// Figlet-style "SOAIM" banner. Lines are padded to a common width in
/// `render_banner` so each centers to the same column.
const LOGO: &str = r#" ____   ___    _    ___ __  __
/ ___| / _ \  / \  |_ _|  \/  |
\___ \| | | |/ _ \  | || |\/| |
 ___) | |_| / ___ \ | || |  | |
|____/ \___/_/   \_\___|_|  |_|"#;

const SUBTITLE: &str = "Know how many times you say *** to your agent";
const REPO: &str = "https://github.com/maximgorbatyuk/swearing-on-ai-meter";

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(8),  // SOAIM logo + subtitle + repo link
        Constraint::Length(12), // activity heatmap
        Constraint::Min(7),     // per-agent comparison (last 14 days)
        Constraint::Length(4),  // filter row + menu
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_heatmap(frame, chunks[1], app);
    render_agents(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let accent = Style::new().fg(Color::Rgb(217, 119, 87)).bold();
    let subtitle_style = Style::new().fg(Color::Rgb(150, 150, 150)).italic();
    let repo_style = Style::new().fg(Color::Rgb(110, 168, 254)).underlined();

    let width = LOGO.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut lines: Vec<Line> = LOGO
        .lines()
        .map(|l| Line::from(Span::styled(format!("{l:<width$}"), accent)).centered())
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(SUBTITLE, subtitle_style)).centered());
    lines.push(Line::from(Span::styled(REPO, repo_style)).centered());

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
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Filter row.
    let mut filter_spans: Vec<Span> = vec![Span::raw("apps: ")];
    filter_spans.push(filter_token(
        "[a]ll",
        app.filter == AppFilter::All,
        Color::White,
    ));
    filter_spans.push(Span::raw(" "));
    let apps = [
        ("[1]claude", Source::ClaudeCode),
        ("[2]desktop", Source::ClaudeDesktop),
        ("[3]codex", Source::CodexCli),
        ("[4]codex-app", Source::CodexApp),
        ("[5]opencode", Source::Opencode),
    ];
    for (label, src) in apps {
        let active = app.filter == AppFilter::One(src);
        filter_spans.push(filter_token(label, active, source_color(src)));
        filter_spans.push(Span::raw(" "));
    }

    let menu = Line::from(vec![
        Span::styled("[9]", Style::new().bold()),
        Span::raw(" 90d  "),
        Span::styled("[0]", Style::new().bold()),
        Span::raw(" 360d  "),
        Span::styled("[d]", Style::new().bold()),
        Span::raw(" 30d  "),
        Span::styled("[r]", Style::new().bold()),
        Span::raw(" refresh  "),
        Span::styled("[q]", Style::new().bold()),
        Span::raw(" quit"),
    ]);

    let para = Paragraph::new(vec![Line::from(filter_spans), menu]);
    frame.render_widget(para, inner);
}

fn filter_token(label: &str, active: bool, color: Color) -> Span<'_> {
    let mut style = Style::new().fg(color);
    if active {
        style = style.bold().underlined();
    }
    Span::styled(label.to_string(), style)
}
