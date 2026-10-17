//! Security and compliance posture.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::security::{summarize, Check, Verdict};
use crate::theme;
use crate::ui::widgets::{card, render_table, TableSpec};

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &mut AppState) {
    if area.is_empty() {
        return;
    }
    let detail_h = if area.height >= 14 {
        5
    } else if area.height >= 9 {
        3
    } else {
        0
    };
    let summary_h = if area.height >= 6 { 1 } else { 0 };
    let mut constraints = vec![Constraint::Min(3)];
    if detail_h > 0 {
        constraints.push(Constraint::Length(detail_h));
    }
    if summary_h > 0 {
        constraints.insert(0, Constraint::Length(summary_h));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut i = 0;
    if summary_h > 0 {
        f.render_widget(
            Paragraph::new(summary_line(app)).style(theme::base()),
            chunks[i],
        );
        i += 1;
    }
    let table_area = chunks[i];
    i += 1;

    let wide = table_area.width >= 100;
    let widths = if wide {
        vec![
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Fill(1),
            Constraint::Length(6),
        ]
    } else {
        vec![
            Constraint::Length(16),
            Constraint::Fill(1),
            Constraint::Length(6),
        ]
    };
    let header = if wide {
        Row::new(["Group", "Check", "Value", ""].map(Cell::from))
    } else {
        Row::new(["Check", "Value", ""].map(Cell::from))
    };
    let rows: Vec<Row> = if app.snapshot.security.is_empty() {
        vec![Row::new(vec![Cell::from(Span::styled(
            "Collecting platform checks…",
            theme::label(),
        ))])]
    } else {
        app.snapshot
            .security
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let repeated = i > 0 && app.snapshot.security[i - 1].group == c.group;
                check_row(c, wide, !repeated)
            })
            .collect()
    };
    let count = app.snapshot.security.len();
    render_table(
        f,
        table_area,
        TableSpec {
            title: "Security",
            header,
            rows,
            widths,
            row_count: count,
        },
        &mut app.security,
    );

    if detail_h > 0 {
        f.render_widget(
            Paragraph::new(detail(app))
                .block(card("Detail"))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(theme::panel()),
            chunks[i],
        );
    }
}

fn summary_line(app: &AppState) -> Line<'static> {
    let s = summarize(&app.snapshot.security);
    if app.snapshot.security.is_empty() {
        return Line::from(Span::styled(" platform checks pending", theme::label()));
    }
    Line::from(vec![
        Span::styled(" ", theme::base()),
        count_chip(s.good, "pass", theme::good()),
        Span::styled("  ", theme::base()),
        count_chip(s.warn, "warn", theme::warn()),
        Span::styled("  ", theme::base()),
        count_chip(s.bad, "fail", theme::bad()),
        Span::styled("  ", theme::base()),
        count_chip(s.info, "info", theme::muted()),
    ])
}

fn count_chip(n: usize, word: &str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!(" {n} {word} "),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn check_row(c: &Check, wide: bool, show_group: bool) -> Row<'static> {
    let mark = Span::styled(
        verdict_mark(c.verdict),
        Style::default().fg(verdict_color(c.verdict)),
    );
    let value = Span::styled(c.value.clone(), theme::value());
    let name = Span::styled(c.name.to_string(), theme::strong());
    let cells: Vec<Cell> = if wide {
        vec![
            Cell::from(Span::styled(
                if show_group { c.group } else { "" },
                theme::label(),
            )),
            Cell::from(name),
            Cell::from(value),
            Cell::from(mark),
        ]
    } else {
        vec![Cell::from(name), Cell::from(value), Cell::from(mark)]
    };
    Row::new(cells).height(1)
}

fn verdict_mark(v: Verdict) -> &'static str {
    if v == Verdict::Info {
        "·"
    } else {
        "●"
    }
}

fn verdict_color(v: Verdict) -> ratatui::style::Color {
    match v {
        Verdict::Good => theme::good(),
        Verdict::Warn => theme::warn(),
        Verdict::Bad => theme::bad(),
        Verdict::Info => theme::muted(),
    }
}

fn detail(app: &AppState) -> Vec<Line<'static>> {
    let Some(c) = app
        .snapshot
        .security
        .get(app.security.selected().unwrap_or(0))
    else {
        return vec![Line::from(Span::styled(
            "Platform checks appear after the first inventory refresh.",
            theme::label(),
        ))];
    };
    vec![Line::from(vec![
        Span::styled(format!("{}  {}  ", c.group, c.name), theme::strong()),
        Span::styled(
            c.verdict.word(),
            Style::default().fg(verdict_color(c.verdict)),
        ),
        Span::styled(format!("  —  {}", c.note), theme::value()),
    ])]
}
