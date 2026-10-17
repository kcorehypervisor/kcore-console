//! GPUs and accelerators: PCI display (class 03) and accelerator (class 12) functions.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::AppState;
use crate::inventory::gpu::Gpu;
use crate::theme;
use crate::ui::widgets::{card, kv_text, render_table, TableSpec};

#[derive(Clone, Copy)]
enum Col {
    Slot,
    Vendor,
    Model,
    Class,
    Driver,
    Group,
    Link,
    Vram,
    Util,
    State,
}

impl Col {
    fn header(self) -> &'static str {
        match self {
            Col::Slot => "Slot",
            Col::Vendor => "Vendor",
            Col::Model => "Model",
            Col::Class => "Class",
            Col::Driver => "Driver",
            Col::Group => "IOMMU",
            Col::Link => "Link",
            Col::Vram => "VRAM",
            Col::Util => "Busy",
            Col::State => "State",
        }
    }

    fn constraint(self) -> Constraint {
        match self {
            Col::Slot => Constraint::Length(12),
            Col::Vendor => Constraint::Length(10),
            Col::Model => Constraint::Fill(1),
            Col::Class => Constraint::Length(7),
            Col::Driver => Constraint::Length(10),
            Col::Group => Constraint::Length(5),
            Col::Link => Constraint::Length(14),
            Col::Vram => Constraint::Length(10),
            Col::Util => Constraint::Length(5),
            Col::State => Constraint::Length(12),
        }
    }
}

fn columns(width: u16) -> &'static [Col] {
    if width >= 150 {
        &[
            Col::Slot,
            Col::Vendor,
            Col::Model,
            Col::Class,
            Col::Driver,
            Col::Group,
            Col::Link,
            Col::Vram,
            Col::Util,
            Col::State,
        ]
    } else if width >= 100 {
        &[
            Col::Slot,
            Col::Model,
            Col::Driver,
            Col::Group,
            Col::Vram,
            Col::State,
        ]
    } else {
        &[Col::Model, Col::Driver, Col::State]
    }
}

pub fn draw(f: &mut Frame<'_>, area: Rect, app: &mut AppState) {
    if area.is_empty() {
        return;
    }
    let detail_h = if area.height >= 16 {
        7
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

    let cols = columns(chunks[0].width.saturating_sub(3));
    let widths: Vec<Constraint> = cols.iter().map(|c| c.constraint()).collect();
    let header = Row::new(cols.iter().map(|c| Cell::from(c.header())));
    let rows: Vec<Row> = if app.snapshot.gpus.is_empty() {
        vec![Row::new(vec![Cell::from(Span::styled(
            "No display or accelerator devices (PCI class 03 / 12)",
            theme::label(),
        ))])]
    } else {
        app.snapshot.gpus.iter().map(|g| gpu_row(g, cols)).collect()
    };
    let count = app.snapshot.gpus.len();
    render_table(
        f,
        chunks[0],
        TableSpec {
            title: "GPU",
            header,
            rows,
            widths,
            row_count: count,
        },
        &mut app.gpu,
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

fn gpu_row(g: &Gpu, cols: &[Col]) -> Row<'static> {
    let cells = cols.iter().map(|c| {
        let (text, style): (String, Style) = match c {
            Col::Slot => (g.slot.clone(), theme::value()),
            Col::Vendor => (g.vendor.clone(), theme::value()),
            Col::Model => (g.model.clone(), theme::strong()),
            Col::Class => (g.class.clone(), theme::value()),
            Col::Driver => (g.driver.clone(), driver_style(g)),
            Col::Group => (g.iommu_group.clone(), theme::value()),
            Col::Link => (g.link.clone(), theme::value()),
            Col::Vram => (vram(g), theme::value()),
            Col::Util => (g.utilization.clone(), theme::value()),
            Col::State => (g.state().to_string(), state_style(g)),
        };
        Cell::from(Span::styled(text, style))
    });
    Row::new(cells).height(1)
}

fn vram(g: &Gpu) -> String {
    g.vram_total.clone()
}

fn driver_style(g: &Gpu) -> Style {
    if g.passthrough() {
        Style::default().fg(theme::accent())
    } else if g.driver == "—" {
        Style::default().fg(theme::warn())
    } else {
        theme::value()
    }
}

fn state_style(g: &Gpu) -> Style {
    match g.state() {
        "passthrough" => Style::default().fg(theme::accent()),
        "unbound" => Style::default().fg(theme::warn()),
        _ => Style::default().fg(theme::good()),
    }
}

fn detail(app: &AppState) -> Vec<Line<'static>> {
    let Some(g) = app.snapshot.gpus.get(app.gpu.selected().unwrap_or(0)) else {
        return vec![Line::from(Span::styled(
            "No GPU or accelerator selected. Devices bound to vfio-pci are reserved for VM passthrough.",
            theme::label(),
        ))];
    };
    let boot = if g.boot_vga { "yes" } else { "no" };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(g.model.clone(), theme::strong()),
            Span::styled(format!("   {}", g.slot), theme::label()),
        ]),
        kv_text(
            "PCI",
            8,
            format!("{}:{}  class {}", g.vendor_id, g.device_id, g.class),
        ),
        kv_text("Vendor", 8, g.vendor.clone()),
        kv_text("Driver", 8, format!("{}  {}", g.driver, g.driver_version)),
        kv_text(
            "IOMMU",
            8,
            format!("group {}    NUMA {}", g.iommu_group, g.numa_node),
        ),
        kv_text("Link", 8, format!("{}   max {}", g.link, g.max_link)),
        kv_text(
            "Memory",
            8,
            format!(
                "{} used / {}    busy {}    {}",
                g.vram_used, g.vram_total, g.utilization, g.temperature
            ),
        ),
        kv_text("DRM", 8, format!("{}    boot VGA {boot}", g.drm)),
    ];
    if g.passthrough() {
        lines.push(Line::from(Span::styled(
            "Bound to vfio-pci: reserved for a guest. The IOMMU group must not be shared with a host device.",
            theme::value().fg(theme::accent()),
        )));
    } else if g.iommu_group == "—" {
        lines.push(Line::from(Span::styled(
            "No IOMMU group. Passthrough of this device is not possible until the IOMMU is enabled.",
            theme::value().fg(theme::warn()),
        )));
    }
    lines
}
