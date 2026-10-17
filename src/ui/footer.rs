//! Key legend and refresh status.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use crate::ui::widgets::key_chip;
use crate::REFRESH_SECS;

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if area.is_empty() {
        return;
    }
    let status = status_text(app);
    let parts = Layout::horizontal([
        Constraint::Min(10),
        Constraint::Length((status.chars().count() as u16 + 2).min(area.width / 2)),
    ])
    .split(area);

    f.render_widget(Paragraph::new(keys(app)).style(theme::base()), parts[0]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(status, theme::label())))
            .right_aligned()
            .style(theme::base()),
        parts[1],
    );
}

fn keys(app: &AppState) -> Line<'static> {
    let mut spans = vec![
        key_chip("1-7"),
        Span::styled(" page ", theme::label()),
        key_chip("Tab"),
        Span::styled(" ", theme::base()),
        key_chip("↑↓"),
        Span::styled(" move ", theme::label()),
        key_chip("r"),
        Span::styled(" refresh ", theme::label()),
        key_chip("?"),
        Span::styled(" help", theme::label()),
    ];
    if app.dev {
        spans.push(Span::styled(" ", theme::base()));
        spans.push(key_chip("q"));
        spans.push(Span::styled(" quit", theme::label().fg(theme::warn())));
    }
    if let Some(msg) = &app.last_msg {
        spans.push(Span::styled(
            format!("  {msg}"),
            theme::health_style("Degraded"),
        ));
    }
    Line::from(spans)
}

fn status_text(app: &AppState) -> String {
    if app.refreshing {
        "refreshing…".to_string()
    } else if let Some(t) = &app.last_refresh {
        format!("updated {t} · {REFRESH_SECS}s")
    } else {
        "waiting for inventory".to_string()
    }
}
