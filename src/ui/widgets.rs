//! Shared building blocks: cards, key/value rows, badges, status dots, key chips.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, Padding, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState,
};
use ratatui::Frame;

use crate::inventory::system::Health;
use crate::theme;

/// Bordered panel with an accent title and the elevated panel fill.
pub fn card(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()).bg(theme::panel_bg()))
        .title(Line::from(vec![
            Span::styled(" ", Style::default().fg(theme::accent_dim())),
            Span::styled(title.to_string(), theme::title_style()),
            Span::styled(" ", Style::default().fg(theme::accent_dim())),
        ]))
        .padding(Padding::horizontal(1))
        .style(theme::panel())
}

/// A card whose border is brighter: used for the panel that owns keyboard focus.
pub fn focused_card(title: &str) -> Block<'static> {
    card(title).border_style(Style::default().fg(theme::accent()).bg(theme::panel_bg()))
}

/// One `label  value` row with the label padded to `label_w`.
pub fn kv(label: &str, label_w: usize, value: Span<'static>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<label_w$}"), theme::label()),
        value,
    ])
}

/// `label  value` row where the value is plain text.
pub fn kv_text(label: &str, label_w: usize, value: impl Into<String>) -> Line<'static> {
    kv(label, label_w, Span::styled(value.into(), theme::value()))
}

/// Inverted colour badge such as ` OK ` or ` DEGRADED `.
pub fn badge(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default()
            .fg(theme::bg())
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

/// Keyboard chip such as ` Tab `.
pub fn key_chip(key: &str) -> Span<'static> {
    Span::styled(format!(" {key} "), theme::key_chip())
}

pub fn health_color(h: &Health) -> Color {
    match h {
        Health::Ok => theme::good(),
        Health::Degraded | Health::Unknown => theme::warn(),
        Health::Critical => theme::bad(),
    }
}

pub fn health_label(h: &Health) -> &'static str {
    match h {
        Health::Ok => "OK",
        Health::Degraded => "DEGRADED",
        Health::Unknown => "UNKNOWN",
        Health::Critical => "CRITICAL",
    }
}

/// Colour for a service status string from the Diagnostics inventory.
pub fn service_color(status: &str) -> Color {
    if status == "healthy" {
        theme::good()
    } else if status.starts_with("failed") {
        theme::bad()
    } else if status.starts_with('—') {
        theme::muted()
    } else {
        theme::warn()
    }
}

/// Colour for a NIC operstate.
pub fn link_color(state: &str) -> Color {
    match state.to_ascii_uppercase().as_str() {
        "UP" => theme::good(),
        "DOWN" | "LOWERLAYERDOWN" => theme::bad(),
        _ => theme::muted(),
    }
}

/// Vertical scrollbar on the right edge of `area` when `total` exceeds `visible`.
pub fn scrollbar(f: &mut Frame<'_>, area: Rect, total: usize, visible: usize, position: usize) {
    if total <= visible || area.height < 2 {
        return;
    }
    let mut state = ScrollbarState::new(total.saturating_sub(visible))
        .position(position.min(total.saturating_sub(visible)))
        .viewport_content_length(visible);
    let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .track_style(Style::default().fg(theme::accent()).bg(theme::panel_bg()))
        .thumb_style(
            Style::default()
                .fg(ratatui::style::Color::Yellow)
                .bg(theme::panel_bg()),
        )
        .begin_style(
            Style::default()
                .fg(ratatui::style::Color::Yellow)
                .bg(theme::panel_bg()),
        )
        .end_style(
            Style::default()
                .fg(ratatui::style::Color::Yellow)
                .bg(theme::panel_bg()),
        );
    f.render_stateful_widget(bar, area, &mut state);
}

/// Capacity bar: `████░░░░ 64%`. `width` is the number of block cells, not counting the percentage.
pub fn usage_bar(pct: u8, width: usize) -> Line<'static> {
    let width = width.max(1);
    let pct = pct.min(100);
    let filled = (pct as usize * width) / 100;
    let color = if pct >= 90 {
        theme::bad()
    } else if pct >= 75 {
        theme::warn()
    } else {
        theme::good()
    };
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled(
            "░".repeat(width - filled),
            Style::default().fg(theme::muted()),
        ),
        Span::styled(format!(" {pct:>3}%"), theme::value()),
    ])
}

/// Everything [`render_table`] needs besides the frame, the area, and the selection.
pub struct TableSpec<'a> {
    pub title: &'a str,
    pub header: Row<'a>,
    pub rows: Vec<Row<'a>>,
    pub widths: Vec<Constraint>,
    pub row_count: usize,
}

/// Selectable table with a stable highlight column and a scrollbar when it overflows.
pub fn render_table(f: &mut Frame<'_>, area: Rect, spec: TableSpec<'_>, state: &mut TableState) {
    let TableSpec {
        title,
        header,
        rows,
        widths,
        row_count,
    } = spec;
    let table = Table::new(rows, widths)
        .header(header.style(theme::table_header()).height(1))
        .column_spacing(1)
        .row_highlight_style(theme::row_selected())
        .highlight_symbol("▌")
        .highlight_spacing(HighlightSpacing::Always)
        .block(focused_card(title))
        .style(theme::panel());
    f.render_stateful_widget(table, area, state);
    let visible = area.height.saturating_sub(3) as usize;
    scrollbar(f, area, row_count, visible, state.offset());
}
