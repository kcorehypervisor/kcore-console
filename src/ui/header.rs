//! Wordmark banner: the kcore logo, identity, and the health badge.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use crate::ui::logo::{self, Logo};
use crate::ui::widgets::{badge, health_color, health_label};

/// Banner height for a terminal of `rows` lines.
pub fn height_for(rows: u16) -> u16 {
    if rows >= 40 {
        6
    } else if rows >= 30 {
        4
    } else if rows >= 24 {
        3
    } else {
        1
    }
}

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if area.is_empty() {
        return;
    }
    let logo_w = logo::variant_for(area.width, area.height)
        .map(logo::width_of)
        .unwrap_or(logo::WORDMARK.len() as u16)
        .min(area.width.saturating_sub(28));
    let badge_w = 12u16.min(area.width.saturating_sub(logo_w + 2));
    let parts = Layout::horizontal([
        Constraint::Length(logo_w),
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(badge_w),
    ])
    .split(area);

    f.render_widget(Logo, parts[0]);
    f.render_widget(
        Paragraph::new(identity(app, parts[2].width, area.height)).style(theme::base()),
        parts[2],
    );
    f.render_widget(
        Paragraph::new(badge_lines(app, area.height))
            .right_aligned()
            .style(theme::base()),
        parts[3],
    );
}

fn identity(app: &AppState, width: u16, rows: u16) -> Vec<Line<'static>> {
    let m = &app.snapshot.meta;
    let mut lines = Vec::new();
    let mut brand = vec![Span::styled(
        m.kind.brand_word(),
        theme::strong().fg(theme::accent()),
    )];
    let tagline = format!("  {}", m.kind.tagline());
    if width as usize > m.kind.brand_word().len() + tagline.len() {
        brand.push(Span::styled(tagline, theme::label()));
    }
    lines.push(Line::from(brand));

    if rows >= 2 {
        let mut host = vec![Span::styled(m.hostname.clone(), theme::strong())];
        let place = match (m.cluster_name.as_str(), m.node_role.as_str()) {
            ("—", "—") | ("", "") => String::new(),
            ("—", role) => format!("  {role}"),
            (cluster, "—") => format!("  {cluster}"),
            (cluster, role) => format!("  {cluster} · {role}"),
        };
        if !place.is_empty() && width as usize > m.hostname.len() + place.len() {
            host.push(Span::styled(place, theme::label()));
        }
        lines.push(Line::from(host));
    }
    if rows >= 3 {
        lines.push(Line::from(Span::styled(
            format!("v{}  ·  build {}", m.version, m.build_id),
            theme::label(),
        )));
    }
    if rows >= 4 {
        lines.push(Line::from(vec![
            Span::styled("up ", theme::label()),
            Span::styled(m.uptime_str.clone(), theme::value()),
            Span::styled(format!("   {}", m.local_time), theme::label()),
        ]));
    }
    if rows >= 5 {
        lines.push(Line::from(Span::styled(
            m.management_url.clone(),
            theme::value(),
        )));
    }
    lines.truncate(rows as usize);
    lines
}

fn badge_lines(app: &AppState, rows: u16) -> Vec<Line<'static>> {
    let m = &app.snapshot.meta;
    let mut lines = vec![Line::from(badge(
        health_label(&m.health),
        health_color(&m.health),
    ))];
    if rows >= 3 && app.dev {
        lines.push(Line::from(badge("DEV", theme::warn())));
    }
    lines
}
