//! Overview: identity cards over the system log.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::api::ApiStatus;
use crate::inventory::logs::LogLine;
use crate::product::Product;
use crate::theme;
use crate::ui::widgets::{
    badge, card, health_color, health_label, kv, kv_text, scrollbar, usage_bar,
};

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if area.is_empty() {
        return;
    }
    let log_min = 4u16;
    let wide = area.width >= 108;
    // The narrow card grows by two rows on a vault host so the repository line is visible.
    let preferred = if wide || app.snapshot.meta.kind == Product::Vault {
        11
    } else {
        9
    };
    let cards_h = if area.height > log_min + 6 {
        preferred.min(area.height - log_min)
    } else {
        area.height
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(cards_h), Constraint::Min(0)])
        .split(area);

    draw_cards(f, chunks[0], app);
    if chunks.len() > 1 && chunks[1].height > 0 {
        draw_log(f, chunks[1], app);
    }
}

fn draw_cards(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let wide = area.width >= 108 && area.height >= 8;
    if !wide {
        f.render_widget(status_card(app, area.width), area);
        return;
    }
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Length(1),
        Constraint::Ratio(1, 3),
        Constraint::Length(1),
        Constraint::Ratio(1, 3),
    ])
    .split(area);
    f.render_widget(node_card(app), cols[0]);
    if app.snapshot.meta.kind == Product::Vault {
        f.render_widget(repository_card(app, cols[2].width), cols[2]);
    } else {
        f.render_widget(health_card(app), cols[2]);
    }
    f.render_widget(access_card(app), cols[4]);
}

fn node_card(app: &AppState) -> Paragraph<'static> {
    let m = &app.snapshot.meta;
    Paragraph::new(vec![
        kv_text("Node", 10, m.hostname.clone()),
        kv_text("Cluster", 10, m.cluster_name.clone()),
        kv_text("Role", 10, m.node_role.clone()),
        kv_text("Version", 10, m.version.clone()),
        kv_text("Build", 10, m.build_id.clone()),
        kv_text("Uptime", 10, m.uptime_str.clone()),
        kv_text("Time", 10, m.local_time.clone()),
    ])
    .block(card("Node status"))
    .style(theme::panel())
}

fn health_card(app: &AppState) -> Paragraph<'static> {
    let m = &app.snapshot.meta;
    let (api_word, api_color) = api_style(&m.api_status);
    let services = service_summary(app);
    Paragraph::new(vec![
        kv(
            "Health",
            10,
            badge(health_label(&m.health), health_color(&m.health)),
        ),
        kv_text("API", 10, m.api_endpoint.clone()),
        kv(
            "Probe",
            10,
            Span::styled(api_word, Style::default().fg(api_color)),
        ),
        kv("Services", 10, services),
    ])
    .block(card("Health"))
    .style(theme::panel())
}

fn access_card(app: &AppState) -> Paragraph<'static> {
    let m = &app.snapshot.meta;
    Paragraph::new(vec![
        kv_text("Manage", 8, m.management_url.clone()),
        Line::from(Span::styled("Remote", theme::label())),
        Line::from(Span::styled(
            m.remote_hint.clone(),
            theme::value().fg(theme::accent()),
        )),
        kv(
            "Login",
            8,
            Span::styled(m.local_login.to_string(), theme::value().fg(theme::warn())),
        ),
        Line::from(Span::styled(
            "administration is remote-only",
            theme::label(),
        )),
    ])
    .block(card("Access"))
    .wrap(Wrap { trim: false })
    .style(theme::panel())
}

/// Single card used when three columns will not fit (80-column consoles).
fn status_card(app: &AppState, _width: u16) -> Paragraph<'static> {
    let m = &app.snapshot.meta;
    let (api_word, api_color) = api_style(&m.api_status);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<10}", "Node"), theme::label()),
            Span::styled(m.hostname.clone(), theme::strong()),
            Span::styled(format!("   {}", m.node_role), theme::value()),
            Span::styled(format!("   {}", m.cluster_name), theme::label()),
        ]),
        kv_text(
            "Version",
            10,
            format!("{}   build {}", m.version, m.build_id),
        ),
        kv_text("Uptime", 10, format!("{}   {}", m.uptime_str, m.local_time)),
        Line::from(vec![
            Span::styled(format!("{:<10}", "Health"), theme::label()),
            badge(health_label(&m.health), health_color(&m.health)),
            Span::styled("   API ", theme::label()),
            Span::styled(m.api_endpoint.clone(), theme::value()),
            Span::styled("  ", theme::label()),
            Span::styled(api_word, Style::default().fg(api_color)),
        ]),
        kv_text("Manage", 10, m.management_url.clone()),
        kv(
            "Remote",
            10,
            Span::styled(m.remote_hint.clone(), theme::value().fg(theme::accent())),
        ),
        kv(
            "Login",
            10,
            Span::styled(
                format!("{} — administration is remote-only", m.local_login),
                theme::value().fg(theme::warn()),
            ),
        ),
    ];
    if m.kind == Product::Vault {
        let repo = &app.snapshot.repository;
        let cap = match repo.used_pct() {
            Some(_pct) => format!("{} used, {} free", repo.used_text(), repo.avail_text()),
            None => "repository not mounted".into(),
        };
        lines.push(kv_text("Backup", 10, format!("{}  {cap}", repo.path)));
    }
    Paragraph::new(lines)
        .block(card("Node status"))
        .style(theme::panel())
}

fn repository_card(app: &AppState, width: u16) -> Paragraph<'static> {
    let repo = &app.snapshot.repository;
    let mut lines = vec![
        kv_text("Path", 8, repo.path.clone()),
        kv_text("Device", 8, repo.source.clone()),
        kv_text("FS", 8, format!("{}  at {}", repo.fstype, repo.mountpoint)),
    ];
    match repo.used_pct() {
        Some(pct) => {
            let bar_w = (width as usize).saturating_sub(14).clamp(8, 24);
            lines.push(kv_text(
                "Size",
                8,
                format!("{} used / {}", repo.used_text(), repo.total_text()),
            ));
            lines.push(kv_text("Free", 8, repo.avail_text()));
            lines.push(usage_bar(pct, bar_w));
        }
        None => {
            lines.push(Line::from(Span::styled(
                if repo.present {
                    "size unavailable"
                } else {
                    "repository directory not found"
                },
                theme::label().fg(theme::warn()),
            )));
        }
    }
    Paragraph::new(lines)
        .block(card("Backup repository"))
        .style(theme::panel())
}

fn api_style(status: &ApiStatus) -> (&'static str, ratatui::style::Color) {
    match status {
        ApiStatus::Unavailable => ("unavailable", theme::warn()),
        ApiStatus::Reachable { healthy: true } => ("available", theme::good()),
        ApiStatus::Reachable { healthy: false } => ("degraded", theme::warn()),
    }
}

fn service_summary(app: &AppState) -> Span<'static> {
    let diag = &app.snapshot.diag;
    if diag.is_empty() {
        return Span::styled("—", theme::label());
    }
    let healthy = diag.iter().filter(|s| s.status == "healthy").count();
    let color = if healthy == diag.len() {
        theme::good()
    } else if healthy == 0 {
        theme::bad()
    } else {
        theme::warn()
    };
    Span::styled(
        format!("{healthy}/{} healthy", diag.len()),
        Style::default().fg(color),
    )
}

fn draw_log(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let logs = &app.snapshot.logs;
    let inner = area.height.saturating_sub(2) as usize;
    let (first, last) = window(logs.len(), inner, app.log_scroll);
    let lines: Vec<Line> = if logs.is_empty() {
        vec![Line::from(Span::styled(
            "No recent journal lines.",
            theme::label(),
        ))]
    } else {
        logs[first..last].iter().map(log_line).collect()
    };
    f.render_widget(
        Paragraph::new(lines)
            .block(card("System log"))
            .style(theme::panel()),
        area,
    );
    scrollbar(f, area, logs.len(), inner, first);
}

fn window(total: usize, visible: usize, scroll: usize) -> (usize, usize) {
    let visible = visible.max(1);
    let first = total.saturating_sub(visible.saturating_add(scroll));
    let last = (first + visible).min(total);
    (first, last)
}

fn log_line(line: &LogLine) -> Line<'static> {
    let text = &line.text;
    match text.split_once(' ') {
        Some((ts, rest)) if ts.contains('T') && ts.len() >= 20 => Line::from(vec![
            Span::styled(ts.to_string(), theme::label()),
            Span::styled(format!(" {rest}"), message_style(rest)),
        ]),
        _ => Line::from(Span::styled(text.clone(), message_style(text))),
    }
}

fn message_style(text: &str) -> Style {
    let l = text.to_ascii_lowercase();
    if l.contains("error") || l.contains(" fail") || l.contains("critical") {
        Style::default().fg(theme::bad())
    } else if l.contains("warn") {
        Style::default().fg(theme::warn())
    } else if l.contains("kernel:") {
        theme::label()
    } else {
        theme::value()
    }
}
