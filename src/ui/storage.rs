//! Block device table and the selected disk.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::disk::Disk;
use crate::theme;
use crate::ui::cols::{disk_columns_for_width, DiskColumn};
use crate::ui::widgets::{card, kv_text, render_table, TableSpec};

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &mut AppState) {
    if area.is_empty() {
        return;
    }
    let detail_h = if area.height >= 14 {
        6
    } else if area.height >= 8 {
        3
    } else {
        0
    };
    let chunks = split(area, detail_h);

    let inner_w = chunks[0].width.saturating_sub(3);
    let cols = disk_columns_for_width(inner_w);
    let widths: Vec<Constraint> = cols.iter().map(|c| c.constraint()).collect();
    let header = Row::new(cols.iter().map(|c| Cell::from(c.header())));
    let rows: Vec<Row> = if app.snapshot.disks.is_empty() {
        vec![Row::new(vec![Cell::from(Span::styled(
            "No disks reported",
            theme::label(),
        ))])]
    } else {
        app.snapshot
            .disks
            .iter()
            .map(|d| disk_row(d, cols))
            .collect()
    };
    let count = app.snapshot.disks.len();
    render_table(
        f,
        chunks[0],
        TableSpec {
            title: "Storage",
            header,
            rows,
            widths,
            row_count: count,
        },
        &mut app.storage,
    );

    if detail_h > 0 {
        f.render_widget(
            Paragraph::new(detail(app))
                .block(card("Detail"))
                .style(theme::panel()),
            chunks[1],
        );
    }
}

fn split(area: Rect, detail_h: u16) -> std::rc::Rc<[Rect]> {
    if detail_h == 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(detail_h)])
            .split(area)
    }
}

fn disk_row(d: &Disk, cols: &[DiskColumn]) -> Row<'static> {
    let cells = cols.iter().map(|c| {
        let (text, style): (String, Style) = match c {
            DiskColumn::Name => (d.name.clone(), theme::strong()),
            DiskColumn::Path => (d.path.clone(), theme::value()),
            DiskColumn::Model => (d.model.clone(), theme::value()),
            DiskColumn::Serial => (d.serial.clone(), theme::value()),
            DiskColumn::Size => (d.size_text.clone(), theme::value()),
            DiskColumn::Kind => (d.kind.clone(), kind_style(&d.kind)),
            DiskColumn::Ro => (d.ro.clone(), theme::value()),
            DiskColumn::Mounts => (d.mountpoints.clone(), theme::value()),
            DiskColumn::Health => (d.health.clone(), health_style(&d.health)),
            DiskColumn::Role => (d.usage_role.clone(), role_style(&d.usage_role)),
        };
        Cell::from(Span::styled(text, style))
    });
    Row::new(cells).height(1)
}

fn kind_style(kind: &str) -> Style {
    match kind {
        "NVMe" => Style::default().fg(theme::accent()),
        "SSD" => theme::value(),
        "HDD" => theme::label(),
        _ => theme::value(),
    }
}

fn role_style(role: &str) -> Style {
    if role.starts_with("system") {
        Style::default().fg(theme::accent())
    } else if role.contains("VM") || role.contains("hint") {
        Style::default().fg(theme::good())
    } else {
        theme::label()
    }
}

fn health_style(health: &str) -> Style {
    let l = health.to_ascii_lowercase();
    if l.contains("fail") || l.contains("crit") {
        Style::default().fg(theme::bad())
    } else if l == "—" || l.is_empty() || l.contains("running") || l == "ok" {
        theme::value()
    } else {
        Style::default().fg(theme::warn())
    }
}

fn detail(app: &AppState) -> Vec<Line<'static>> {
    let Some(d) = app.snapshot.disks.get(app.storage_sel()) else {
        return vec![Line::from(Span::styled("No disk selected", theme::label()))];
    };
    vec![
        Line::from(vec![
            Span::styled(d.name.clone(), theme::strong()),
            Span::styled(format!("  {}", d.path), theme::label()),
            Span::styled(format!("   {}", d.size_text), theme::value()),
        ]),
        kv_text("Model", 8, format!("{}    {}", d.model, d.serial)),
        kv_text("Kind", 8, format!("{}    {}", d.kind, d.ro)),
        kv_text("Mounts", 8, d.mountpoints.clone()),
        kv_text("Role", 8, d.usage_role.clone()),
        kv_text("Health", 8, d.health.clone()),
    ]
}
