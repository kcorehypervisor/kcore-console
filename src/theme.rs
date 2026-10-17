//! Color palette aligned with `assets/kcore-logo.png`: black field, violet accent, silver text.

use ratatui::style::{Color, Modifier, Style};

pub fn bg() -> Color {
    Color::Rgb(0, 0, 0)
}

pub fn accent() -> Color {
    // Dominant violet from the shipped README / console logo art (sampled from PNG)
    Color::Rgb(83, 90, 244)
}

/// Dimmer violet for rules, inactive borders, and separators.
pub fn accent_dim() -> Color {
    Color::Rgb(52, 56, 140)
}

/// Highlight / metallic tone for wordmarks (logo silver).
pub fn silver() -> Color {
    Color::Rgb(210, 212, 220)
}

pub fn text() -> Color {
    Color::Rgb(228, 230, 235)
}

pub fn muted() -> Color {
    Color::Rgb(118, 120, 132)
}

/// Selected row / active tab background.
pub fn selection_bg() -> Color {
    Color::Rgb(30, 32, 64)
}

/// Slightly elevated panel fill (status cards).
pub fn panel_bg() -> Color {
    Color::Rgb(10, 10, 16)
}

/// Active tab: inverted accent chip so the current page is unmistakable.
pub fn tab_active() -> Style {
    Style::default()
        .fg(bg())
        .bg(accent())
        .add_modifier(Modifier::BOLD)
}

/// Inactive tab foreground.
pub fn tab_inactive() -> Style {
    Style::default().fg(muted()).bg(bg())
}

pub fn good() -> Color {
    Color::Rgb(80, 200, 120)
}

pub fn warn() -> Color {
    Color::Rgb(255, 200, 100)
}

pub fn bad() -> Color {
    Color::Rgb(255, 100, 100)
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
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
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
        .fg(silver())
        .bg(selection_bg())
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
        .fg(silver())
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
