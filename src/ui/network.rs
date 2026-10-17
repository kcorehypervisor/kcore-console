//! Network interface table and the selected interface.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::network::Nic;
use crate::theme;
use crate::ui::cols::{net_columns_for_width, net_iface_shows_master_suffix, NetColumn};
use crate::ui::widgets::{card, kv_text, link_color, render_table, TableSpec};

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &mut AppState) {
    if area.is_empty() {
        return;
    }
    let detail_h = if area.height >= 18 {
        8
    } else if area.height >= 10 {
        5
    } else {
        0
    };
    let chunks = if detail_h == 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(detail_h)])
            .split(area)
    };

    let inner_w = chunks[0].width.saturating_sub(3);
    let cols = net_columns_for_width(inner_w);
    let narrow = inner_w < 110;
    let master_suffix = net_iface_shows_master_suffix(cols);
    let widths: Vec<Constraint> = cols.iter().map(|c| c.constraint(narrow)).collect();
    let header = Row::new(cols.iter().map(|c| Cell::from(header_of(*c, narrow))));

    let rows: Vec<Row> = if app.snapshot.nics.is_empty() {
        vec![Row::new(vec![Cell::from(Span::styled(
            "No interfaces found",
            theme::label(),
        ))])]
    } else {
        app.snapshot
            .nics
            .iter()
            .map(|n| nic_row(n, cols, master_suffix))
            .collect()
    };
    let count = app.snapshot.nics.len();
    render_table(
        f,
        chunks[0],
        TableSpec {
            title: "Network",
            header,
            rows,
            widths,
            row_count: count,
        },
        &mut app.network,
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

fn header_of(col: NetColumn, narrow: bool) -> &'static str {
    if narrow {
        match col {
            NetColumn::Def => "d",
            NetColumn::Mgmt => "m",
            other => other.header(),
        }
    } else {
        col.header()
    }
}

fn nic_row(n: &Nic, cols: &[NetColumn], master_suffix: bool) -> Row<'static> {
    let (ipv6_primary, ipv6_extra) = n.primary_ipv6_display();
    let ipv6 = if ipv6_extra > 0 {
        format!("{ipv6_primary} +{ipv6_extra}")
    } else {
        ipv6_primary
    };
    let cells = cols.iter().map(|c| {
        let (text, style) = match c {
            NetColumn::Iface => (n.iface_label(master_suffix), theme::strong()),
            NetColumn::State => (
                n.oper_state.clone(),
                Style::default().fg(link_color(&n.oper_state)),
            ),
            NetColumn::Mac => (n.mac.clone(), theme::value()),
            NetColumn::Ipv4 => (n.ipv4.clone(), theme::value()),
            NetColumn::Ipv6 => (ipv6.clone(), theme::value()),
            NetColumn::Mtu => (n.mtu.clone(), theme::value()),
            NetColumn::Speed => (n.speed.clone(), theme::value()),
            NetColumn::Driver => (n.driver.clone(), theme::value()),
            NetColumn::Master => (
                if n.master.is_empty() {
                    "—".into()
                } else {
                    n.master.clone()
                },
                theme::value(),
            ),
            NetColumn::Def => flag(n.default_route),
            NetColumn::Mgmt => flag(n.management),
        };
        Cell::from(Span::styled(text, style))
    });
    Row::new(cells).height(1)
}

fn flag(on: bool) -> (String, Style) {
    if on {
        ("●".into(), Style::default().fg(theme::accent()))
    } else {
        ("·".into(), theme::label())
    }
}

fn detail(app: &AppState) -> Vec<Line<'static>> {
    let Some(n) = app.snapshot.nics.get(app.network_sel()) else {
        return vec![Line::from(Span::styled(
            "No interface selected",
            theme::label(),
        ))];
    };
    let mut marks = Vec::new();
    if n.default_route {
        marks.push("default route");
    }
    if n.management {
        marks.push("management");
    }
    if !n.master.is_empty() {
        marks.push("bridge member");
    }
    let marks = if marks.is_empty() {
        "—".to_string()
    } else {
        marks.join(" · ")
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(n.name.clone(), theme::strong()),
            Span::styled(
                format!("  {}", n.oper_state),
                Style::default().fg(link_color(&n.oper_state)),
            ),
            Span::styled(format!("   {marks}"), theme::label()),
        ]),
        kv_text("MAC", 8, format!("{}    driver {}", n.mac, n.driver)),
        kv_text("Link", 8, format!("MTU {}    speed {}", n.mtu, n.speed)),
        kv_text("IPv4", 8, n.ipv4.clone()),
    ];
    if n.ipv6 == "—" || n.ipv6.is_empty() {
        lines.push(kv_text("IPv6", 8, "—"));
    } else {
        for (i, addr) in n.ipv6.split(',').map(str::trim).enumerate() {
            if i == 0 {
                lines.push(kv_text("IPv6", 8, addr.to_string()));
            } else {
                lines.push(kv_text("", 8, addr.to_string()));
            }
        }
    }
    if !n.master.is_empty() {
        lines.push(kv_text("master", 8, n.master.clone()));
    }
    lines
}
