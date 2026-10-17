//! GPU and accelerator inventory from sysfs PCI (class 03 display, class 12 accelerator),
//! enriched with amdgpu sysfs counters and `nvidia-smi` when present.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::format::format_bytes;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gpu {
    /// PCI slot, e.g. `0000:0d:00.0`.
    pub slot: String,
    pub vendor: String,
    pub model: String,
    pub vendor_id: String,
    pub device_id: String,
    /// `VGA`, `3D`, `Display`, or `Accel`.
    pub class: String,
    pub driver: String,
    pub iommu_group: String,
    pub numa_node: String,
    /// `x16 16 GT/s` (current link), or `—`.
    pub link: String,
    pub max_link: String,
    /// DRM nodes such as `card1 renderD128`.
    pub drm: String,
    pub boot_vga: bool,
    pub vram_total: String,
    pub vram_used: String,
    pub utilization: String,
    pub temperature: String,
    pub driver_version: String,
}

impl Gpu {
    /// Bound to `vfio-pci`, i.e. reserved for VM passthrough.
    pub fn passthrough(&self) -> bool {
        self.driver == "vfio-pci"
    }

    /// Short state word for tables.
    pub fn state(&self) -> &'static str {
        if self.passthrough() {
            "passthrough"
        } else if self.driver == "—" {
            "unbound"
        } else {
            "host"
        }
    }
}

/// Vendor and device names from a `pci.ids` file; empty when none is installed.
#[derive(Debug, Default)]
pub struct PciIds {
    vendors: HashMap<u16, String>,
    devices: HashMap<(u16, u16), String>,
}

impl PciIds {
    pub fn parse(text: &str) -> Self {
        let mut ids = Self::default();
        let mut vendor: Option<u16> = None;
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if line.starts_with("\t\t") {
                continue;
            }
            if let Some(rest) = line.strip_prefix('\t') {
                let (Some(v), Some((id, name))) = (vendor, split_id(rest)) else {
                    continue;
                };
                ids.devices.insert((v, id), name);
            } else if let Some((id, name)) = split_id(line) {
                if line.starts_with('C') {
                    break;
                }
                vendor = Some(id);
                ids.vendors.insert(id, name);
            }
        }
        ids
    }

    /// Load from `KCORE_PCI_IDS` or the usual hwdata locations.
    pub fn load() -> Self {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("KCORE_PCI_IDS") {
            candidates.push(PathBuf::from(p));
        }
        candidates.push("/usr/share/hwdata/pci.ids".into());
        candidates.push("/usr/share/misc/pci.ids".into());
        candidates.push("/run/current-system/sw/share/hwdata/pci.ids".into());
        for c in candidates {
            if let Ok(text) = std::fs::read_to_string(&c) {
                return Self::parse(&text);
            }
        }
        Self::default()
    }

    pub fn vendor(&self, id: u16) -> String {
        self.vendors
            .get(&id)
            .cloned()
            .unwrap_or_else(|| builtin_vendor(id).to_string())
    }

    pub fn device(&self, vendor: u16, device: u16) -> String {
        self.devices
            .get(&(vendor, device))
            .cloned()
            .unwrap_or_else(|| format!("device {device:04x}"))
    }
}

fn split_id(line: &str) -> Option<(u16, String)> {
    let (id, name) = line.split_once(char::is_whitespace)?;
    let id = u16::from_str_radix(id.trim(), 16).ok()?;
    Some((id, name.trim().to_string()))
}

fn builtin_vendor(id: u16) -> &'static str {
    match id {
        0x10de => "NVIDIA",
        0x1002 => "AMD",
        0x8086 => "Intel",
        0x1a03 => "ASPEED",
        0x102b => "Matrox",
        0x15ad => "VMware",
        0x1234 => "QEMU",
        0x1af4 => "Red Hat (virtio)",
        0x1013 => "Cirrus",
        0x1da3 => "Habana",
        0x1e52 => "Tenstorrent",
        _ => "unknown vendor",
    }
}

fn class_label(class: u32) -> Option<&'static str> {
    match class >> 8 {
        0x0300 => Some("VGA"),
        0x0301 => Some("XGA"),
        0x0302 => Some("3D"),
        0x0380 => Some("Display"),
        0x1200 | 0x1201 | 0x1280 => Some("Accel"),
        _ => None,
    }
}

fn read_trim(p: &Path) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_hex(p: &Path) -> Option<u32> {
    let s = read_trim(p)?;
    u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

fn link_name(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_link(dir.join(name))
        .ok()
        .and_then(|l| l.file_name().map(|f| f.to_string_lossy().to_string()))
}

fn link_text(dir: &Path, speed: &str, width: &str) -> String {
    let speed = read_trim(&dir.join(speed)).map(|s| s.replace(" PCIe", ""));
    let width = read_trim(&dir.join(width));
    match (width, speed) {
        (Some(w), Some(s)) => format!("x{w} {s}"),
        (Some(w), None) => format!("x{w}"),
        (None, Some(s)) => s,
        (None, None) => "—".into(),
    }
}

fn drm_nodes(dir: &Path) -> String {
    let Ok(rd) = std::fs::read_dir(dir.join("drm")) else {
        return "—".into();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("card") || n.starts_with("render"))
        .collect();
    names.sort();
    if names.is_empty() {
        "—".into()
    } else {
        names.join(" ")
    }
}

fn hwmon_temp(dir: &Path) -> Option<String> {
    let rd = std::fs::read_dir(dir.join("hwmon")).ok()?;
    for e in rd.flatten() {
        if let Some(v) = read_trim(&e.path().join("temp1_input")) {
            if let Ok(milli) = v.parse::<i64>() {
                return Some(format!("{}°C", milli / 1000));
            }
        }
    }
    None
}

fn bytes_text(dir: &Path, name: &str) -> Option<String> {
    read_trim(&dir.join(name))
        .and_then(|s| s.parse::<u64>().ok())
        .map(format_bytes)
}

/// Enumerate display and accelerator functions under a `/sys/bus/pci/devices`-like directory.
pub(crate) fn scan_pci(devices_dir: &Path, ids: &PciIds) -> Vec<Gpu> {
    let Ok(rd) = std::fs::read_dir(devices_dir) else {
        return Vec::new();
    };
    let mut slots: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    slots.sort();
    let mut out = Vec::new();
    for dir in slots {
        let Some(class) = read_hex(&dir.join("class")) else {
            continue;
        };
        let Some(class_name) = class_label(class) else {
            continue;
        };
        let vendor_id = read_hex(&dir.join("vendor")).unwrap_or(0) as u16;
        let device_id = read_hex(&dir.join("device")).unwrap_or(0) as u16;
        let slot = dir
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(Gpu {
            slot,
            vendor: ids.vendor(vendor_id),
            model: ids.device(vendor_id, device_id),
            vendor_id: format!("{vendor_id:04x}"),
            device_id: format!("{device_id:04x}"),
            class: class_name.to_string(),
            driver: link_name(&dir, "driver").unwrap_or_else(|| "—".into()),
            iommu_group: link_name(&dir, "iommu_group").unwrap_or_else(|| "—".into()),
            numa_node: read_trim(&dir.join("numa_node"))
                .filter(|n| n != "-1")
                .unwrap_or_else(|| "—".into()),
            link: link_text(&dir, "current_link_speed", "current_link_width"),
            max_link: link_text(&dir, "max_link_speed", "max_link_width"),
            drm: drm_nodes(&dir),
            boot_vga: read_trim(&dir.join("boot_vga")).as_deref() == Some("1"),
            vram_total: bytes_text(&dir, "mem_info_vram_total").unwrap_or_else(|| "—".into()),
            vram_used: bytes_text(&dir, "mem_info_vram_used").unwrap_or_else(|| "—".into()),
            utilization: read_trim(&dir.join("gpu_busy_percent"))
                .map(|p| format!("{p}%"))
                .unwrap_or_else(|| "—".into()),
            temperature: hwmon_temp(&dir).unwrap_or_else(|| "—".into()),
            driver_version: "—".into(),
        });
    }
    out
}

/// One `nvidia-smi` CSV row, keyed by lower-case PCI slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmiRow {
    pub name: String,
    pub mem_total_mib: u64,
    pub mem_used_mib: u64,
    pub util: String,
    pub temp: String,
    pub driver_version: String,
}

/// Parse `--query-gpu=pci.bus_id,name,memory.total,memory.used,utilization.gpu,temperature.gpu,driver_version --format=csv,noheader,nounits`.
pub(crate) fn parse_nvidia_smi(text: &str) -> HashMap<String, SmiRow> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() < 7 {
            continue;
        }
        // nvidia-smi prints 00000000:01:00.0; sysfs uses 0000:01:00.0.
        let bus = f[0].to_ascii_lowercase();
        let slot = bus
            .split_once(':')
            .map(|(dom, rest)| {
                let dom = dom.trim_start_matches('0');
                format!("{:0>4}:{rest}", dom)
            })
            .unwrap_or(bus);
        out.insert(
            slot,
            SmiRow {
                name: f[1].to_string(),
                mem_total_mib: f[2].parse().unwrap_or(0),
                mem_used_mib: f[3].parse().unwrap_or(0),
                util: format!("{}%", f[4]),
                temp: format!("{}°C", f[5]),
                driver_version: f[6].to_string(),
            },
        );
    }
    out
}

pub(crate) fn apply_smi(gpus: &mut [Gpu], rows: &HashMap<String, SmiRow>) {
    for g in gpus.iter_mut() {
        let Some(r) = rows.get(&g.slot.to_ascii_lowercase()) else {
            continue;
        };
        if !r.name.is_empty() {
            g.model = r.name.clone();
        }
        if r.mem_total_mib > 0 {
            g.vram_total = format_bytes(r.mem_total_mib * 1024 * 1024);
            g.vram_used = format_bytes(r.mem_used_mib * 1024 * 1024);
        }
        g.utilization = r.util.clone();
        g.temperature = r.temp.clone();
        g.driver_version = r.driver_version.clone();
    }
}

fn run_nvidia_smi() -> Option<String> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=pci.bus_id,name,memory.total,memory.used,utilization.gpu,temperature.gpu,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Host GPUs and accelerators, best effort.
pub fn list_gpus() -> Vec<Gpu> {
    let ids = PciIds::load();
    let mut gpus = scan_pci(Path::new("/sys/bus/pci/devices"), &ids);
    if gpus.iter().any(|g| g.vendor_id == "10de") {
        if let Some(text) = run_nvidia_smi() {
            apply_smi(&mut gpus, &parse_nvidia_smi(&text));
        }
    }
    gpus
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: &str = "# comment\n1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t1638  Cezanne [Radeon Vega Series]\n\t\t1043 1234  Some board\n10de  NVIDIA Corporation\n\t2684  AD102 [GeForce RTX 4090]\nC 03  Display controller\n\t00  VGA compatible controller\n";

    fn fake_pci(root: &Path) {
        let dev = root.join("0000:0d:00.0");
        std::fs::create_dir_all(dev.join("drm/card1")).unwrap();
        std::fs::create_dir_all(dev.join("drm/renderD128")).unwrap();
        std::fs::create_dir_all(dev.join("hwmon/hwmon3")).unwrap();
        std::fs::create_dir_all(root.join("groups/2")).unwrap();
        std::fs::create_dir_all(root.join("drivers/amdgpu")).unwrap();
        for (k, v) in [
            ("class", "0x030000"),
            ("vendor", "0x1002"),
            ("device", "0x1638"),
            ("numa_node", "-1"),
            ("current_link_speed", "8.0 GT/s PCIe"),
            ("current_link_width", "16"),
            ("max_link_speed", "16.0 GT/s PCIe"),
            ("max_link_width", "16"),
            ("boot_vga", "1"),
            ("mem_info_vram_total", "536870912"),
            ("mem_info_vram_used", "268435456"),
            ("gpu_busy_percent", "7"),
        ] {
            std::fs::write(dev.join(k), v).unwrap();
        }
        std::fs::write(dev.join("hwmon/hwmon3/temp1_input"), "43000").unwrap();
        std::os::unix::fs::symlink("../groups/2", dev.join("iommu_group")).unwrap();
        std::os::unix::fs::symlink("../drivers/amdgpu", dev.join("driver")).unwrap();

        let nic = root.join("0000:05:00.0");
        std::fs::create_dir_all(&nic).unwrap();
        std::fs::write(nic.join("class"), "0x020000").unwrap();

        let nv = root.join("0000:41:00.0");
        std::fs::create_dir_all(&nv).unwrap();
        std::fs::create_dir_all(root.join("drivers/vfio-pci")).unwrap();
        std::fs::write(nv.join("class"), "0x030200").unwrap();
        std::fs::write(nv.join("vendor"), "0x10de").unwrap();
        std::fs::write(nv.join("device"), "0x2684").unwrap();
        std::os::unix::fs::symlink("../drivers/vfio-pci", nv.join("driver")).unwrap();
    }

    #[test]
    fn pci_ids_parse_vendor_and_device_names() {
        let ids = PciIds::parse(IDS);
        assert_eq!(ids.vendor(0x1002), "Advanced Micro Devices, Inc. [AMD/ATI]");
        assert_eq!(ids.device(0x1002, 0x1638), "Cezanne [Radeon Vega Series]");
        assert_eq!(ids.device(0x10de, 0x2684), "AD102 [GeForce RTX 4090]");
        assert_eq!(ids.device(0x10de, 0xffff), "device ffff");
        assert_eq!(PciIds::default().vendor(0x10de), "NVIDIA");
    }

    #[test]
    fn scans_display_and_accelerator_functions_only() {
        let tmp = tempfile::tempdir().unwrap();
        fake_pci(tmp.path());
        let gpus = scan_pci(tmp.path(), &PciIds::parse(IDS));
        assert_eq!(gpus.len(), 2, "{gpus:?}");
        let amd = &gpus[0];
        assert_eq!(amd.slot, "0000:0d:00.0");
        assert_eq!(amd.class, "VGA");
        assert_eq!(amd.driver, "amdgpu");
        assert_eq!(amd.iommu_group, "2");
        assert_eq!(amd.link, "x16 8.0 GT/s");
        assert_eq!(amd.max_link, "x16 16.0 GT/s");
        assert_eq!(amd.drm, "card1 renderD128");
        assert_eq!(amd.vram_total, "512.0 MiB");
        assert_eq!(amd.vram_used, "256.0 MiB");
        assert_eq!(amd.utilization, "7%");
        assert_eq!(amd.temperature, "43°C");
        assert_eq!(amd.numa_node, "—");
        assert!(amd.boot_vga);
        assert_eq!(amd.state(), "host");

        let nv = &gpus[1];
        assert_eq!(nv.class, "3D");
        assert!(nv.passthrough());
        assert_eq!(nv.state(), "passthrough");
        assert_eq!(nv.iommu_group, "—");
    }

    #[test]
    fn nvidia_smi_rows_enrich_matching_slot() {
        let rows = parse_nvidia_smi(
            "00000000:41:00.0, NVIDIA GeForce RTX 4090, 24564, 1024, 12, 51, 550.54.14\n",
        );
        assert!(rows.contains_key("0000:41:00.0"), "{rows:?}");
        let mut gpus = vec![Gpu {
            slot: "0000:41:00.0".into(),
            vram_total: "—".into(),
            ..Default::default()
        }];
        apply_smi(&mut gpus, &rows);
        assert_eq!(gpus[0].model, "NVIDIA GeForce RTX 4090");
        assert_eq!(gpus[0].vram_total, "24.0 GiB");
        assert_eq!(gpus[0].vram_used, "1.0 GiB");
        assert_eq!(gpus[0].utilization, "12%");
        assert_eq!(gpus[0].temperature, "51°C");
        assert_eq!(gpus[0].driver_version, "550.54.14");
    }
}
