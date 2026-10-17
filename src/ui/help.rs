//! Keys and the notes for this appliance.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use crate::ui::widgets::{card, key_chip};

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if area.is_empty() {
        return;
    }
    let notes = notes(app);
    if area.width >= 100 && area.height >= 12 {
        let cols =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
        f.render_widget(keys_panel(), cols[0]);
        f.render_widget(notes, cols[1]);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Min(4)])
            .split(area);
        f.render_widget(keys_panel(), rows[0]);
        f.render_widget(notes, rows[1]);
    }
}

fn keys_panel() -> Paragraph<'static> {
    Paragraph::new(vec![
        row("1-7", "jump to a page"),
        row("Tab", "next page"),
        row("Shift+Tab", "previous page"),
        row("← →", "previous / next page"),
        row("↑ ↓", "move the selection, or scroll the log"),
        row("j k", "same as the arrow keys"),
        row("PgUp PgDn", "move by ten rows"),
        row("Home End", "first or last row"),
        row("g G", "oldest or newest log line"),
        row("r", "refresh inventory now"),
        row("? h", "open this page"),
        row("Esc", "back to Overview"),
        row("q", "quit — development mode only"),
    ])
    .block(card("Keys"))
    .style(theme::panel())
}

fn row(key: &str, what: &str) -> Line<'static> {
    Line::from(vec![
        key_chip(key),
        Span::styled(format!("  {what}"), theme::value()),
    ])
}

fn notes(app: &AppState) -> Paragraph<'static> {
    let text = app.snapshot.meta.kind.help_notes().join("\n\n");
    Paragraph::new(text)
        .block(card(app.snapshot.meta.kind.label()))
        .wrap(Wrap { trim: true })
        .style(theme::panel())
}
