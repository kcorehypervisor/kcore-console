//! Local service status for the active product.

use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::api::ApiStatus;
use crate::inventory::diagnostics::ServiceLine;
use crate::theme;
use crate::ui::widgets::{render_table, service_color, TableSpec};

pub fn draw(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &AppState) {
    if area.is_empty() {
        return;
    }
    let wide = area.width >= 90;
    let widths = if wide {
        vec![
            Constraint::Length(3),
            Constraint::Length(22),
            Constraint::Length(16),
            Constraint::Fill(1),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(18),
        ]
    };
    let header = if wide {
        Row::new(["", "Service", "Status", "Unit"].map(Cell::from))
    } else {
        Row::new(["", "Service", "Status"].map(Cell::from))
    };

    let healthy = app
        .snapshot
        .diag
        .iter()
        .filter(|s| s.status == "healthy")
        .count();
    let total = app.snapshot.diag.len();
    let title = if total == 0 {
        "Diagnostics".to_string()
    } else {
        format!("Diagnostics  {healthy}/{total} healthy")
    };

    let mut rows: Vec<Row> = Vec::new();
    rows.push(api_row(
        &app.snapshot.meta.api_status,
        &app.snapshot.meta.api_endpoint,
        wide,
    ));
    if app.snapshot.diag.is_empty() {
        rows.push(Row::new(vec![Cell::from(Span::styled(
            "No product services reported",
            theme::label(),
        ))]));
    } else {
        for s in &app.snapshot.diag {
            rows.push(service_row(s, wide));
        }
    }

    // The API row is informational and not part of the selectable service list,
    // so the table is rendered stateless: selection lives on the other pages.
    let table_rows = rows;
    let count = table_rows.len();
    let mut state = ratatui::widgets::TableState::default();
    render_table(
        f,
        area,
        TableSpec {
            title: &title,
            header,
            rows: table_rows,
            widths,
            row_count: count,
        },
        &mut state,
    );
}

fn api_row(status: &ApiStatus, endpoint: &str, wide: bool) -> Row<'static> {
    let (word, color) = match status {
        ApiStatus::Unavailable => ("unavailable", theme::warn()),
        ApiStatus::Reachable { healthy: true } => ("available", theme::good()),
        ApiStatus::Reachable { healthy: false } => ("degraded", theme::warn()),
    };
    let dot = Span::styled("●", Style::default().fg(color));
    let name = Span::styled("local API", theme::strong());
    let status_cell = Cell::from(Span::styled(word, Style::default().fg(color)));
    let cells: Vec<Cell> = if wide {
        vec![
            Cell::from(dot),
            Cell::from(name),
            status_cell,
            Cell::from(endpoint.to_string()),
        ]
    } else {
        vec![Cell::from(dot), Cell::from(name), status_cell]
    };
    Row::new(cells).height(1)
}

fn service_row(s: &ServiceLine, wide: bool) -> Row<'static> {
    let color = service_color(&s.status);
    let cells: Vec<Cell> = if wide {
        vec![
            Cell::from(Span::styled("●", Style::default().fg(color))),
            Cell::from(Span::styled(s.name.clone(), theme::strong())),
            Cell::from(Span::styled(s.status.clone(), Style::default().fg(color))),
            Cell::from(Span::styled(s.unit.clone(), theme::label())),
        ]
    } else {
        vec![
            Cell::from(Span::styled("●", Style::default().fg(color))),
            Cell::from(Span::styled(s.name.clone(), theme::strong())),
            Cell::from(Span::styled(
                short_status(&s.status),
                Style::default().fg(color),
            )),
        ]
    };
    Row::new(cells).height(1)
}

fn short_status(status: &str) -> String {
    if status == "healthy" {
        "healthy".into()
    } else if let Some((word, _)) = status.split_once(' ') {
        word.to_string()
    } else {
        status.to_string()
    }
}
