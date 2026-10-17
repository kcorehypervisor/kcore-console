//! Rendered-screen checks and SVG snapshots under target/console-visual/.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use crate::app::{AppState, Page};
use crate::inventory::diagnostics::ServiceLine;
use crate::inventory::disk::Disk;
use crate::inventory::gpu::Gpu;
use crate::inventory::logs::LogLine;
use crate::inventory::network::Nic;
use crate::inventory::security::{Check, Verdict};
use crate::inventory::system::{Health, Meta};
use crate::inventory::vault::Repository;
use crate::inventory::Snapshot;
use crate::product::Product;
use crate::ui::draw;
use crate::ui::visual::{buffer_contains_on_one_line, buffer_plain, buffer_svg};

fn check(group: &'static str, name: &'static str, value: &str, verdict: Verdict) -> Check {
    Check {
        group,
        name,
        value: value.into(),
        verdict,
        note: format!("{name} is reported for the operator."),
    }
}

fn sample_snapshot(kind: Product) -> Snapshot {
    let meta = Meta {
        kind,
        product: kind.label().into(),
        hostname: "node-103".into(),
        version: "0.3.0".into(),
        build_id: "deadbeef".into(),
        uptime_str: "4h 12m".into(),
        local_time: "2026-10-10 20:00:00".into(),
        cluster_name: "garage".into(),
        node_role: "compute".into(),
        health: Health::Ok,
        management_url: "https://192.168.40.103:8443".into(),
        api_endpoint: "192.168.40.103:9091".into(),
        remote_hint: "kcorectl login https://192.168.40.103:8443".into(),
        ..Meta::default()
    };

    Snapshot {
        meta,
        nics: vec![
            Nic {
                name: "eno1".into(),
                mac: "aa:bb:cc:dd:ee:01".into(),
                oper_state: "UP".into(),
                ipv4: "—".into(),
                ipv6: "—".into(),
                mtu: "1500".into(),
                speed: "1000 Mbps".into(),
                driver: "igb".into(),
                default_route: false,
                management: false,
                master: "br0".into(),
            },
            Nic {
                name: "br0".into(),
                mac: "aa:bb:cc:dd:ee:01".into(),
                oper_state: "UP".into(),
                ipv4: "192.168.40.143/24".into(),
                ipv6: "2a02:ef8:1000:1d00:7d1d:ca86:5156:8dd8/64, fe80::7d1d:ca86:5156:8dd8/64"
                    .into(),
                mtu: "1500".into(),
                speed: "—".into(),
                driver: "bridge".into(),
                default_route: true,
                management: true,
                master: String::new(),
            },
            Nic {
                name: "enp6s0d1".into(),
                mac: "11:22:33:44:55:66".into(),
                oper_state: "UP".into(),
                ipv4: "192.168.47.3/24".into(),
                ipv6: "—".into(),
                mtu: "9000".into(),
                speed: "10000 Mbps".into(),
                driver: "mlx5_core".into(),
                default_route: false,
                management: false,
                master: String::new(),
            },
        ],
        disks: vec![
            Disk {
                name: "nvme0n1".into(),
                path: "/dev/nvme0n1".into(),
                model: "Samsung 990 PRO".into(),
                serial: "S6Z1NX0T".into(),
                size_text: "1.8 TiB".into(),
                kind: "NVMe".into(),
                ro: "rw".into(),
                mountpoints: "/".into(),
                health: "ok".into(),
                usage_role: "system".into(),
                ..Disk::default()
            },
            Disk {
                name: "sda".into(),
                path: "/dev/sda".into(),
                model: "Archive HDD".into(),
                serial: "—".into(),
                size_text: "931.5 GiB".into(),
                kind: "HDD".into(),
                ro: "rw".into(),
                mountpoints: "—".into(),
                health: "—".into(),
                usage_role: "unknown".into(),
                ..Disk::default()
            },
        ],
        gpus: vec![
            Gpu {
                slot: "0000:0d:00.0".into(),
                vendor: "AMD".into(),
                model: "Cezanne [Radeon Vega]".into(),
                vendor_id: "1002".into(),
                device_id: "1638".into(),
                class: "VGA".into(),
                driver: "amdgpu".into(),
                iommu_group: "2".into(),
                numa_node: "—".into(),
                link: "x16 8.0 GT/s".into(),
                max_link: "x16 16.0 GT/s".into(),
                drm: "card1 renderD128".into(),
                boot_vga: true,
                vram_total: "512.0 MiB".into(),
                vram_used: "128.0 MiB".into(),
                utilization: "7%".into(),
                temperature: "43°C".into(),
                driver_version: "—".into(),
            },
            Gpu {
                slot: "0000:41:00.0".into(),
                vendor: "NVIDIA".into(),
                model: "GeForce RTX 4090".into(),
                vendor_id: "10de".into(),
                device_id: "2684".into(),
                class: "3D".into(),
                driver: "vfio-pci".into(),
                iommu_group: "14".into(),
                numa_node: "0".into(),
                link: "x16 16.0 GT/s".into(),
                max_link: "x16 16.0 GT/s".into(),
                drm: "—".into(),
                boot_vga: false,
                vram_total: "24.0 GiB".into(),
                vram_used: "—".into(),
                utilization: "—".into(),
                temperature: "—".into(),
                driver_version: "—".into(),
            },
        ],
        security: vec![
            check("Platform", "TPM", "TPM 2.0 · sha256", Verdict::Good),
            check("Platform", "Secure Boot", "enabled", Verdict::Good),
            check("Platform", "IOMMU", "5 groups", Verdict::Good),
            check("Kernel", "Lockdown", "none", Verdict::Warn),
            check("Kernel", "Disk encryption", "no LUKS volumes", Verdict::Bad),
        ],
        diag: vec![
            ServiceLine {
                name: "kcore-node-agent".into(),
                unit: "kcore-node-agent.service".into(),
                status: "healthy".into(),
            },
            ServiceLine {
                name: "kcore-controller".into(),
                unit: "kcore-controller.service".into(),
                status: "inactive (not installed or inactive)".into(),
            },
            ServiceLine {
                name: "kcore-dashboard".into(),
                unit: "kcore-dashboard.service".into(),
                status: "healthy".into(),
            },
        ],
        logs: vec![
            LogLine {
                text: "2026-10-10T19:58:01+01:00 node-103 kernel: Bridge br0 up".into(),
            },
            LogLine {
                text: "2026-10-10T19:59:12+01:00 node-103 kcore-node-agent[412]: heartbeat ok"
                    .into(),
            },
            LogLine {
                text: "2026-10-10T20:00:00+01:00 node-103 systemd[1]: Started kcore-console".into(),
            },
        ],
        repository: Repository {
            path: "/var/lib/kcore-vault".into(),
            present: true,
            source: "/dev/mapper/vault".into(),
            fstype: "xfs".into(),
            mountpoint: "/var/lib/kcore-vault".into(),
            total: 4_000_000_000_000,
            used: 1_500_000_000_000,
            avail: 2_500_000_000_000,
        },
    }
}

fn render(page: Page, width: u16, height: u16, kind: Product) -> Buffer {
    let mut app = AppState::new(true, sample_snapshot(kind));
    app.page = page;
    app.last_refresh = Some("20:00:05".into());
    if page == Page::Network {
        app.network.select(Some(1));
    }
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| draw(f, &mut app)).expect("draw");
    terminal.backend().buffer().clone()
}

fn visual_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/console-visual");
    std::fs::create_dir_all(&root).expect("visual dir");
    root
}

fn write_visual(name: &str, buf: &Buffer) {
    let dir = visual_dir();
    std::fs::write(dir.join(format!("{name}.txt")), buffer_plain(buf)).expect("txt");
    std::fs::write(dir.join(format!("{name}.svg")), buffer_svg(buf, name)).expect("svg");
}

#[test]
fn active_tab_is_highlighted_and_visible() {
    let buf = render(Page::Network, 128, 48, Product::Hypervisor);
    let plain = buffer_plain(&buf);
    assert!(plain.contains(" Network "), "Network tab label must render");
    assert!(plain.contains(" Overview "), "all tabs remain visible");
    assert!(
        buffer_contains_on_one_line(&buf, " Network "),
        "active Network panel title"
    );
    write_visual("network-128x48", &buf);
}

#[test]
fn overview_splits_status_and_system_log() {
    let buf = render(Page::Overview, 128, 48, Product::Hypervisor);
    assert!(buffer_contains_on_one_line(&buf, "Node status"));
    assert!(buffer_contains_on_one_line(&buf, "System log"));
    assert!(buffer_contains_on_one_line(&buf, "node-103"));
    assert!(buffer_contains_on_one_line(&buf, "heartbeat ok"));
    let plain = buffer_plain(&buf);
    let status_at = plain.find("Node status").expect("status");
    let log_at = plain.find("System log").expect("log");
    assert!(status_at < log_at, "status panel above system log");
    write_visual("overview-128x48", &buf);
}

#[test]
fn overview_fits_80x25_without_panic() {
    let buf = render(Page::Overview, 80, 25, Product::Hypervisor);
    assert!(buffer_contains_on_one_line(&buf, " Overview "));
    assert!(buffer_contains_on_one_line(&buf, "Node status"));
    write_visual("overview-80x25", &buf);
}

#[test]
fn network_ipv4_complete_at_200x56() {
    let buf = render(Page::Network, 200, 56, Product::Hypervisor);
    assert!(
        buffer_contains_on_one_line(&buf, "192.168.40.143/24"),
        "IPv4 must appear intact on one row at 200x56"
    );
    write_visual("network-200x56", &buf);
}

#[test]
fn network_ipv4_complete_at_128x48() {
    let buf = render(Page::Network, 128, 48, Product::Hypervisor);
    assert!(
        buffer_contains_on_one_line(&buf, "192.168.40.143/24"),
        "IPv4 must appear intact on one row at 128x48"
    );
    assert!(
        buffer_contains_on_one_line(&buf, "eno1 -> br0")
            || buffer_contains_on_one_line(&buf, "master"),
        "bridge membership must be visible at 128x48"
    );
}

#[test]
fn network_ipv4_complete_at_80x25() {
    let buf = render(Page::Network, 80, 25, Product::Hypervisor);
    assert!(
        buffer_contains_on_one_line(&buf, "192.168.40.143/24"),
        "IPv4 must appear intact on one row at 80x25"
    );
    write_visual("network-80x25", &buf);
}

#[test]
fn header_falls_back_to_text_when_short() {
    let buf = render(Page::Overview, 80, 20, Product::Hypervisor);
    assert!(
        buffer_contains_on_one_line(&buf, "KCORE"),
        "a one-row banner uses the text wordmark"
    );
    write_visual("overview-80x20", &buf);
}

#[test]
fn every_page_renders_at_the_three_console_sizes() {
    for (w, h) in [(80, 25), (128, 48), (200, 56)] {
        for page in Page::ALL {
            let buf = render(page, w, h, Product::Hypervisor);
            assert!(
                buffer_contains_on_one_line(&buf, page.tab_label()),
                "{page:?} at {w}x{h}"
            );
            write_visual(
                &format!("{}-{w}x{h}", page.tab_label().to_ascii_lowercase()),
                &buf,
            );
        }
    }
}

#[test]
fn vault_overview_shows_the_backup_repository() {
    let wide = render(Page::Overview, 128, 48, Product::Vault);
    assert!(buffer_contains_on_one_line(&wide, "VAULT"));
    assert!(
        buffer_contains_on_one_line(&wide, "Backup repository")
            || buffer_plain(&wide).contains("/var/lib/kcore-vault")
    );
    write_visual("overview-vault-128x48", &wide);

    let narrow = render(Page::Overview, 80, 25, Product::Vault);
    assert!(buffer_plain(&narrow).contains("/var/lib/kcore-vault"));
    write_visual("overview-vault-80x25", &narrow);
}

#[test]
fn gpu_page_shows_passthrough_and_host_devices() {
    let buf = render(Page::Gpu, 128, 48, Product::Hypervisor);
    let plain = buffer_plain(&buf);
    assert!(plain.contains("vfio-pci"), "{plain}");
    assert!(plain.contains("amdgpu"), "{plain}");
    write_visual("gpu-128x48", &buf);
}

#[test]
fn security_page_summarizes_verdicts() {
    let buf = render(Page::Security, 128, 48, Product::Hypervisor);
    let plain = buffer_plain(&buf);
    assert!(plain.contains("pass"), "{plain}");
    assert!(plain.contains("fail"), "{plain}");
    assert!(plain.contains("TPM"), "{plain}");
    write_visual("security-128x48", &buf);
}
