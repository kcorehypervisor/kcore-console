//! Screen chrome: banner, tab bar, page body, footer.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::app::{AppState, Page};
use crate::theme;

use super::{diagnostics, footer, gpu, header, help, network, overview, security, storage};

pub fn draw(f: &mut Frame<'_>, app: &mut AppState) {
    let area = f.area();
    f.render_widget(Block::default().style(theme::base()), area);

    let header_h = header::height_for(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    if chunks.len() < 5 {
        return;
    }

    header::draw(f, chunks[0], app);
    draw_tabs(f, chunks[1], app);
    draw_rule(f, chunks[2]);

    match app.page {
        Page::Overview => overview::draw(f, chunks[3], app),
        Page::Network => network::draw(f, chunks[3], app),
        Page::Storage => storage::draw(f, chunks[3], app),
        Page::Gpu => gpu::draw(f, chunks[3], app),
        Page::Security => security::draw(f, chunks[3], app),
        Page::Diagnostics => diagnostics::draw(f, chunks[3], app),
        Page::Help => help::draw(f, chunks[3], app),
    }

    footer::draw(f, chunks[4], app);
}

fn draw_tabs(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let numbered: Vec<String> = Page::ALL
        .iter()
        .map(|p| format!(" {} {} ", p.hotkey(), p.tab_label()))
        .collect();
    let plain: Vec<String> = Page::ALL
        .iter()
        .map(|p| format!(" {} ", p.tab_label()))
        .collect();
    let labels = if line_width(&numbered) <= area.width as usize {
        numbered
    } else {
        plain
    };

    let mut spans = Vec::with_capacity(labels.len() * 2);
    for (i, label) in labels.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("│", theme::base().fg(theme::accent_dim())));
        }
        let style = if i == app.page as usize {
            theme::tab_active()
        } else {
            theme::tab_inactive()
        };
        spans.push(Span::styled(label, style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), area);
}

fn line_width(labels: &[String]) -> usize {
    labels.iter().map(String::len).sum::<usize>() + labels.len().saturating_sub(1)
}

fn draw_rule(f: &mut Frame<'_>, area: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(area.width as usize),
            theme::base().fg(theme::accent()),
        )))
        .style(theme::base()),
        area,
    );
}
