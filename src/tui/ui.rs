//! Dashboard layout and rendering: counter/header, heatmap, the two bar
//! charts, and the filter + menu footer.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use super::app::App;
use super::widgets::{heatmap, hour, source_color, weekday};
use crate::model::Source;
use crate::stats::AppFilter;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // today / header
        Constraint::Length(10), // heatmap (7 weekday rows + legend + borders)
        Constraint::Min(8),     // hour + weekday charts
        Constraint::Length(4),  // filter row + menu
    ])
    .split(frame.area());

    render_header(frame, chunks[0], app);
    render_heatmap(frame, chunks[1], app);
    render_charts(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .title(Line::from(" Swearing on AI Meter ").left_aligned())
        .title(
            Line::from(format!(
                "window: {}  filter: {} ",
                app.window.label(),
                app.filter.label()
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let t = &app.dash.today;
    let mut spans = vec![
        Span::styled(format!("TODAY: {}", t.total()), Style::new().bold()),
        Span::raw("   ("),
    ];
    for (idx, src) in Source::ALL.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::styled(
            format!("{} {}", src.short(), t.per_source[src.index()]),
            Style::new().fg(source_color(*src)),
        ));
    }
    spans.push(Span::raw(")"));
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn render_heatmap(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" Last {} days (per day) ", app.window.days());
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    heatmap::render(frame.buffer_mut(), inner, &app.dash);
}

fn render_charts(frame: &mut Frame, area: Rect, app: &App) {
    let cols =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area);

    let hour_block = Block::bordered().title(" Per hour (0–23) ");
    let hour_inner = hour_block.inner(cols[0]);
    frame.render_widget(hour_block, cols[0]);
    hour::render(frame.buffer_mut(), hour_inner, &app.dash);

    let wd_block = Block::bordered().title(" Per weekday (Mon–Sun) ");
    let wd_inner = wd_block.inner(cols[1]);
    frame.render_widget(wd_block, cols[1]);
    weekday::render(frame.buffer_mut(), wd_inner, &app.dash);
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
