//! Palette for the Linux console. Named ANSI colors stay bright when the VT
//! maps them onto its 16-color palette; 24-bit values were collapsing into black.

use ratatui::style::{Color, Modifier, Style};

pub fn bg() -> Color {
    Color::Black
}

/// Bright blue for borders and the logo K. Dark RGB blue was invisible on tty1.
pub fn accent() -> Color {
    Color::LightBlue
}

/// Separators. Still a real blue, not a near-black dim.
pub fn accent_dim() -> Color {
    Color::Blue
}

/// Wordmark and emphasised text.
pub fn silver() -> Color {
    Color::White
}

pub fn text() -> Color {
    Color::White
}

pub fn muted() -> Color {
    Color::Cyan
}

/// Selected-row background. Yellow on the console is readable; blue was not.
pub fn selection_bg() -> Color {
    Color::Yellow
}

/// Cards sit on the same black field as the rest of the console.
pub fn panel_bg() -> Color {
    Color::Black
}

/// Current page: yellow text on black. A blue chip with black text did not read.
pub fn tab_active() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .bg(bg())
        .add_modifier(Modifier::BOLD)
}

/// Other pages.
pub fn tab_inactive() -> Style {
    Style::default().fg(Color::White).bg(bg())
}

pub fn good() -> Color {
    Color::LightGreen
}

pub fn warn() -> Color {
    Color::Yellow
}

pub fn bad() -> Color {
    Color::LightRed
}

/// Base style for the background field.
pub fn base() -> Style {
    Style::default().fg(text()).bg(bg())
}

/// Base style inside a card.
pub fn panel() -> Style {
    Style::default().fg(text()).bg(panel_bg())
}

pub fn title_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Dim label in a key/value row.
pub fn label() -> Style {
    Style::default().fg(muted())
}

/// Primary value text.
pub fn value() -> Style {
    Style::default().fg(text())
}

/// Emphasised value text.
pub fn strong() -> Style {
    Style::default().fg(silver()).add_modifier(Modifier::BOLD)
}

/// Keyboard chip in the footer and on the Help page.
pub fn key_chip() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .bg(bg())
        .add_modifier(Modifier::BOLD)
}

/// Table header row.
pub fn table_header() -> Style {
    Style::default()
        .fg(accent())
        .bg(panel_bg())
        .add_modifier(Modifier::BOLD)
}

/// Highlighted table row.
pub fn row_selected() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(selection_bg())
        .add_modifier(Modifier::BOLD)
}

pub fn health_style(s: &str) -> Style {
    let c = if s == "OK" {
        good()
    } else if s == "Degraded" {
        warn()
    } else if s == "Critical" {
        bad()
    } else {
        warn()
    };
    Style::default().fg(c).add_modifier(Modifier::BOLD)
}
