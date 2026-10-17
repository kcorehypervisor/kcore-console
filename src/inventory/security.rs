//! Platform security and compliance posture: TPM, Secure Boot, firmware identity, IOMMU,
//! kernel hardening, and encryption at rest. Everything is read from sysfs/procfs.

use std::path::{Path, PathBuf};

/// Traffic-light verdict for one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Meets the baseline.
    Good,
    /// Present but weak, or worth an operator's attention.
    Warn,
    /// Missing or disabled: blocks a compliant posture.
    Bad,
    /// Informational, no judgement.
    Info,
}

impl Verdict {
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Good => "pass",
            Verdict::Warn => "warn",
            Verdict::Bad => "fail",
            Verdict::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// `Platform` or `Kernel`.
    pub group: &'static str,
    pub name: &'static str,
    /// Short value for the table cell.
    pub value: String,
    pub verdict: Verdict,
    /// Longer explanation for the Detail card.
    pub note: String,
}

impl Check {
    fn new(
        group: &'static str,
        name: &'static str,
        value: impl Into<String>,
        verdict: Verdict,
        note: impl Into<String>,
    ) -> Self {
        Self {
            group,
            name,
            value: value.into(),
            verdict,
            note: note.into(),
        }
    }
}

const PLATFORM: &str = "Platform";
const KERNEL: &str = "Kernel";

const EFI_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

fn read_trim(p: &Path) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn list_dir(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Value byte of a boolean EFI variable (4-byte attributes, then data).
fn efivar_bool(efivars: &Path, name: &str) -> Option<bool> {
    let bytes = std::fs::read(efivars.join(format!("{name}-{EFI_GUID}"))).ok()?;
    bytes.get(4).map(|b| *b == 1)
}

/// Collect every check from a filesystem root (`/` on a real host).
pub fn collect(root: &Path) -> Vec<Check> {
    let sys = root.join("sys");
    let proc = root.join("proc");
    let dev = root.join("dev");
    let mut out = Vec::new();

    firmware(&sys, &mut out);
    secure_boot(&sys, &mut out);
    tpm(&sys, &dev, &mut out);
    iommu(&sys, &proc, &mut out);
    virtualization(&proc, &dev, &mut out);
    ecc(&sys, &mut out);
    hwrng(&sys, &mut out);

    kernel(&proc, &mut out);
    lockdown(&sys, &mut out);
    mitigations(&sys, &mut out);
    smt(&sys, &mut out);
    mac(&sys, &mut out);
    module_signing(&sys, &mut out);
    aslr(&proc, &mut out);
    luks(&sys, &mut out);
    out
}

fn firmware(sys: &Path, out: &mut Vec<Check>) {
    let dmi = sys.join("class/dmi/id");
    let get = |n: &str| read_trim(&dmi.join(n)).unwrap_or_else(|| "—".into());
    let uefi = sys.join("firmware/efi").is_dir();
    let mode = if uefi { "UEFI" } else { "Legacy BIOS" };
    out.push(Check::new(
        PLATFORM,
        "Boot firmware",
        format!("{mode} · {} {}", get("bios_vendor"), get("bios_version")),
        if uefi { Verdict::Good } else { Verdict::Warn },
        format!(
            "{mode} firmware {} {} ({}). UEFI is required for Secure Boot and measured boot.",
            get("bios_vendor"),
            get("bios_version"),
            get("bios_date")
        ),
    ));
    out.push(Check::new(
        PLATFORM,
        "System",
        format!("{} {}", get("sys_vendor"), get("product_name")),
        Verdict::Info,
        format!(
            "{} {} · board {} {} · serial {} · chassis serial {}. Record these for the asset register.",
            get("sys_vendor"),
            get("product_name"),
            get("board_vendor"),
            get("board_name"),
            get("product_serial"),
            get("chassis_serial")
        ),
    ));
}

fn secure_boot(sys: &Path, out: &mut Vec<Check>) {
    let efivars = sys.join("firmware/efi/efivars");
    if !sys.join("firmware/efi").is_dir() {
        out.push(Check::new(
            PLATFORM,
            "Secure Boot",
            "not available",
            Verdict::Bad,
            "The host booted in legacy BIOS mode, so firmware cannot verify the boot chain.",
        ));
        return;
    }
    match (
        efivar_bool(&efivars, "SecureBoot"),
        efivar_bool(&efivars, "SetupMode"),
    ) {
        (Some(true), Some(true)) => out.push(Check::new(
            PLATFORM,
            "Secure Boot",
            "enabled, setup mode",
            Verdict::Warn,
            "Secure Boot is on but the firmware is in setup mode: keys can be changed without authentication. Exit setup mode (user mode) to lock the key database.",
        )),
        (Some(true), _) => out.push(Check::new(
            PLATFORM,
            "Secure Boot",
            "enabled",
            Verdict::Good,
            "Firmware verifies the bootloader and kernel signatures against the enrolled key database.",
        )),
        (Some(false), _) => out.push(Check::new(
            PLATFORM,
            "Secure Boot",
            "disabled",
            Verdict::Bad,
            "Secure Boot is off. Unsigned bootloaders and kernels can run; enable it in firmware setup and enrol the kcore keys.",
        )),
        (None, _) => out.push(Check::new(
            PLATFORM,
            "Secure Boot",
            "unknown",
            Verdict::Warn,
            "The SecureBoot EFI variable is not readable. Check that efivarfs is mounted.",
        )),
    }
}

fn tpm(sys: &Path, dev: &Path, out: &mut Vec<Check>) {
    let class = sys.join("class/tpm");
    let chips: Vec<String> = list_dir(&class)
        .into_iter()
        .filter(|n| n.starts_with("tpm") && !n.contains("rm"))
        .collect();
    let Some(chip) = chips.first() else {
        out.push(Check::new(
            PLATFORM,
            "TPM",
            "not present",
            Verdict::Bad,
            "No Trusted Platform Module was found. Measured boot, disk-unlock sealing, and remote attestation are unavailable. Enable fTPM/PTT in firmware or fit a discrete TPM.",
        ));
        return;
    };
    let chip_dir = class.join(chip);
    let major = read_trim(&chip_dir.join("tpm_version_major"));
    let caps = read_trim(&chip_dir.join("device/caps")).unwrap_or_default();
    let version = match major.as_deref() {
        Some("2") => "2.0".to_string(),
        Some("1") => "1.2".to_string(),
        _ if caps.contains("1.2") => "1.2".to_string(),
        _ => "unknown".to_string(),
    };
    let banks: Vec<String> = list_dir(&chip_dir)
        .into_iter()
        .filter_map(|n| n.strip_prefix("pcr-").map(str::to_string))
        .collect();
    let rm = dev
        .join(format!("tpmrm{}", chip.trim_start_matches("tpm")))
        .exists();
    let manufacturer = caps
        .lines()
        .find_map(|l| l.strip_prefix("Manufacturer:"))
        .map(|s| s.trim().to_string());
    let (verdict, summary) = match version.as_str() {
        "2.0" => (
            Verdict::Good,
            "TPM 2.0 meets the baseline for measured boot and key sealing.",
        ),
        "1.2" => (
            Verdict::Warn,
            "TPM 1.2 only supports SHA-1 PCR banks and is deprecated for new deployments.",
        ),
        _ => (Verdict::Warn, "TPM version could not be determined."),
    };
    let mut note = format!("{chip}: TPM {version}. {summary}");
    if !banks.is_empty() {
        note.push_str(&format!(" PCR banks: {}.", banks.join(", ")));
    }
    if let Some(m) = manufacturer {
        note.push_str(&format!(" Manufacturer {m}."));
    }
    note.push_str(if rm {
        " Resource manager /dev/tpmrm0 is available."
    } else {
        " No /dev/tpmrm device: user-space tools must open the chip exclusively."
    });
    let value = if banks.is_empty() {
        format!("TPM {version} ({chip})")
    } else {
        format!("TPM {version} · {}", banks.join("/"))
    };
    out.push(Check::new(PLATFORM, "TPM", value, verdict, note));
}

fn iommu(sys: &Path, proc: &Path, out: &mut Vec<Check>) {
    let groups = list_dir(&sys.join("kernel/iommu_groups")).len();
    let cmdline = read_trim(&proc.join("cmdline")).unwrap_or_default();
    let flags: Vec<&str> = cmdline
        .split_whitespace()
        .filter(|w| w.contains("iommu"))
        .collect();
    if groups > 0 {
        let flag_text = if flags.is_empty() {
            String::new()
        } else {
            format!(" · {}", flags.join(" "))
        };
        out.push(Check::new(
            PLATFORM,
            "IOMMU",
            format!("{groups} groups{flag_text}"),
            Verdict::Good,
            format!(
                "The IOMMU is active with {groups} isolation groups. DMA from devices is confined and PCI passthrough is possible for isolated groups.{}",
                if flags.is_empty() { String::new() } else { format!(" Kernel flags: {}.", flags.join(" ")) }
            ),
        ));
    } else {
        out.push(Check::new(
            PLATFORM,
            "IOMMU",
            "inactive",
            Verdict::Warn,
            "No IOMMU groups are present. Enable VT-d / AMD-Vi in firmware and boot with intel_iommu=on or amd_iommu=on; without it devices can DMA anywhere and GPU passthrough is impossible.",
        ));
    }
}

fn virtualization(proc: &Path, dev: &Path, out: &mut Vec<Check>) {
    let cpuinfo = read_trim(&proc.join("cpuinfo")).unwrap_or_default();
    let flags_line = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .unwrap_or("");
    let ext = if flags_line.contains(" vmx") {
        Some("VT-x")
    } else if flags_line.contains(" svm") {
        Some("AMD-V")
    } else {
        None
    };
    let kvm = dev.join("kvm").exists();
    let (value, verdict, note) = match (ext, kvm) {
        (Some(e), true) => (
            format!("{e} · /dev/kvm"),
            Verdict::Good,
            format!("{e} is exposed and the KVM device is present, so guests run with hardware isolation."),
        ),
        (Some(e), false) => (
            format!("{e} · no /dev/kvm"),
            Verdict::Warn,
            format!("{e} is advertised but /dev/kvm is missing. Load the kvm module or check firmware virtualisation settings."),
        ),
        (None, _) => (
            "not available".to_string(),
            Verdict::Bad,
            "Neither VT-x nor AMD-V is visible. Enable virtualisation in firmware; VMs cannot run with hardware isolation otherwise.".to_string(),
        ),
    };
    out.push(Check::new(PLATFORM, "Virtualization", value, verdict, note));
}

fn ecc(sys: &Path, out: &mut Vec<Check>) {
    let mcs = list_dir(&sys.join("devices/system/edac/mc"))
        .into_iter()
        .filter(|n| n.starts_with("mc"))
        .count();
    if mcs > 0 {
        out.push(Check::new(
            PLATFORM,
            "ECC memory",
            format!("{mcs} controller(s)"),
            Verdict::Good,
            "EDAC reports memory controllers with error detection; single-bit errors are corrected and logged.",
        ));
    } else {
        out.push(Check::new(
            PLATFORM,
            "ECC memory",
            "not reported",
            Verdict::Info,
            "No EDAC memory controller is registered. Either the memory is non-ECC or the EDAC driver is not loaded.",
        ));
    }
}

fn hwrng(sys: &Path, out: &mut Vec<Check>) {
    match read_trim(&sys.join("class/misc/hw_random/rng_current")) {
        Some(cur) if cur != "none" => out.push(Check::new(
            PLATFORM,
            "Hardware RNG",
            cur.clone(),
            Verdict::Good,
            format!("Hardware random source {cur} feeds the kernel entropy pool."),
        )),
        _ => out.push(Check::new(
            PLATFORM,
            "Hardware RNG",
            "none",
            Verdict::Info,
            "No hardware RNG is selected. The kernel still has RDRAND/RDSEED and jitter entropy on modern CPUs.",
        )),
    }
}

fn kernel(proc: &Path, out: &mut Vec<Check>) {
    let release = read_trim(&proc.join("sys/kernel/osrelease")).unwrap_or_else(|| "—".into());
    let cpuinfo = read_trim(&proc.join("cpuinfo")).unwrap_or_default();
    let microcode = cpuinfo
        .lines()
        .find(|l| l.starts_with("microcode"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().to_string());
    let model = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "—".into());
    out.push(Check::new(
        KERNEL,
        "Kernel",
        release.clone(),
        Verdict::Info,
        format!(
            "Linux {release} on {model}{}.",
            microcode
                .map(|m| format!(" · microcode {m}"))
                .unwrap_or_default()
        ),
    ));
}

fn lockdown(sys: &Path, out: &mut Vec<Check>) {
    match read_trim(&sys.join("kernel/security/lockdown")) {
        Some(s) => {
            let mode = s
                .split_whitespace()
                .find(|w| w.starts_with('['))
                .map(|w| w.trim_matches(['[', ']']).to_string())
                .unwrap_or_else(|| s.clone());
            let (verdict, note) = match mode.as_str() {
                "confidentiality" => (Verdict::Good, "Lockdown in confidentiality mode: root cannot read kernel memory or load unsigned code."),
                "integrity" => (Verdict::Good, "Lockdown in integrity mode: root cannot modify the running kernel (unsigned modules, kexec, /dev/mem)."),
                _ => (Verdict::Warn, "Kernel lockdown is off. With Secure Boot enabled, lockdown=integrity prevents root from tampering with the kernel."),
            };
            out.push(Check::new(KERNEL, "Lockdown", mode, verdict, note));
        }
        None => out.push(Check::new(
            KERNEL,
            "Lockdown",
            "unsupported",
            Verdict::Info,
            "This kernel does not expose the lockdown LSM.",
        )),
    }
}

fn mitigations(sys: &Path, out: &mut Vec<Check>) {
    let dir = sys.join("devices/system/cpu/vulnerabilities");
    let names = list_dir(&dir);
    if names.is_empty() {
        out.push(Check::new(
            KERNEL,
            "CPU mitigations",
            "unknown",
            Verdict::Info,
            "The kernel does not report CPU vulnerability status.",
        ));
        return;
    }
    let mut vulnerable = Vec::new();
    for n in &names {
        if let Some(v) = read_trim(&dir.join(n)) {
            if v.starts_with("Vulnerable") {
                vulnerable.push(format!("{n} ({v})"));
            }
        }
    }
    if vulnerable.is_empty() {
        out.push(Check::new(
            KERNEL,
            "CPU mitigations",
            format!("{} checked, 0 vulnerable", names.len()),
            Verdict::Good,
            format!(
                "All {} reported CPU vulnerabilities are mitigated or not affected.",
                names.len()
            ),
        ));
    } else {
        out.push(Check::new(
            KERNEL,
            "CPU mitigations",
            format!("{} vulnerable of {}", vulnerable.len(), names.len()),
            Verdict::Warn,
            format!("Unmitigated: {}.", vulnerable.join("; ")),
        ));
    }
}

fn smt(sys: &Path, out: &mut Vec<Check>) {
    match read_trim(&sys.join("devices/system/cpu/smt/active")).as_deref() {
        Some("1") => out.push(Check::new(
            KERNEL,
            "SMT",
            "enabled",
            Verdict::Info,
            "Simultaneous multithreading is on. Disable it (nosmt) if guests of different tenants must never share a core.",
        )),
        Some(_) => out.push(Check::new(
            KERNEL,
            "SMT",
            "disabled",
            Verdict::Good,
            "Simultaneous multithreading is off; sibling-thread side channels do not apply.",
        )),
        None => {}
    }
}

fn mac(sys: &Path, out: &mut Vec<Check>) {
    let lsm = read_trim(&sys.join("kernel/security/lsm")).unwrap_or_default();
    let apparmor = sys.join("kernel/security/apparmor").is_dir() || lsm.contains("apparmor");
    let selinux = sys.join("fs/selinux").is_dir() || lsm.contains("selinux");
    let (value, verdict, note) = if selinux {
        (
            "SELinux",
            Verdict::Good,
            "SELinux mandatory access control is active.".to_string(),
        )
    } else if apparmor {
        (
            "AppArmor",
            Verdict::Good,
            "AppArmor mandatory access control is active.".to_string(),
        )
    } else {
        (
            "none",
            Verdict::Warn,
            format!(
                "No mandatory access control (SELinux/AppArmor) is active. Active LSMs: {}.",
                if lsm.is_empty() {
                    "—".to_string()
                } else {
                    lsm
                }
            ),
        )
    };
    out.push(Check::new(KERNEL, "Access control", value, verdict, note));
}

fn module_signing(sys: &Path, out: &mut Vec<Check>) {
    match read_trim(&sys.join("module/module/parameters/sig_enforce")).as_deref() {
        Some("Y") => out.push(Check::new(
            KERNEL,
            "Module signing",
            "enforced",
            Verdict::Good,
            "Only kernel modules signed with a trusted key can load.",
        )),
        Some(_) => out.push(Check::new(
            KERNEL,
            "Module signing",
            "not enforced",
            Verdict::Warn,
            "Unsigned kernel modules can be loaded by root. Boot with module.sig_enforce=1 or enable lockdown.",
        )),
        None => out.push(Check::new(
            KERNEL,
            "Module signing",
            "unknown",
            Verdict::Info,
            "The kernel does not expose module.sig_enforce.",
        )),
    }
}

fn aslr(proc: &Path, out: &mut Vec<Check>) {
    match read_trim(&proc.join("sys/kernel/randomize_va_space")).as_deref() {
        Some("2") => out.push(Check::new(
            KERNEL,
            "ASLR",
            "full",
            Verdict::Good,
            "Address-space layout randomisation covers stack, mmap, VDSO, and heap.",
        )),
        Some(v) => out.push(Check::new(
            KERNEL,
            "ASLR",
            format!("level {v}"),
            Verdict::Warn,
            "randomize_va_space is below 2; set it to 2 for full randomisation.",
        )),
        None => {}
    }
}

fn luks(sys: &Path, out: &mut Vec<Check>) {
    let block = sys.join("block");
    let mut luks = Vec::new();
    for name in list_dir(&block)
        .into_iter()
        .filter(|n| n.starts_with("dm-"))
    {
        let uuid = read_trim(&block.join(&name).join("dm/uuid")).unwrap_or_default();
        if uuid.starts_with("CRYPT-LUKS") {
            let label = read_trim(&block.join(&name).join("dm/name")).unwrap_or(name);
            luks.push(label);
        }
    }
    if luks.is_empty() {
        out.push(Check::new(
            KERNEL,
            "Disk encryption",
            "no LUKS volumes",
            Verdict::Warn,
            "No active LUKS mapping was found. Data at rest is not encrypted by the OS; confirm self-encrypting drives or enable LUKS for VM and backup storage.",
        ));
    } else {
        out.push(Check::new(
            KERNEL,
            "Disk encryption",
            format!("LUKS: {}", luks.join(", ")),
            Verdict::Good,
            format!("{} LUKS volume(s) active: {}.", luks.len(), luks.join(", ")),
        ));
    }
}

/// Counts per verdict for the summary line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub good: usize,
    pub warn: usize,
    pub bad: usize,
    pub info: usize,
}

pub fn summarize(checks: &[Check]) -> Summary {
    let mut s = Summary::default();
    for c in checks {
        match c.verdict {
            Verdict::Good => s.good += 1,
            Verdict::Warn => s.warn += 1,
            Verdict::Bad => s.bad += 1,
            Verdict::Info => s.info += 1,
        }
    }
    s
}

/// Security checks for the running host.
pub fn security_checks() -> Vec<Check> {
    collect(&PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    fn find<'a>(checks: &'a [Check], name: &str) -> &'a Check {
        checks
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("missing check {name}"))
    }

    #[test]
    fn hardened_host_passes_the_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        write(r, "sys/class/dmi/id/bios_vendor", "LENOVO");
        write(r, "sys/class/dmi/id/bios_version", "M3AKT4EA");
        write(r, "sys/class/dmi/id/bios_date", "01/01/2025");
        write(r, "sys/class/dmi/id/sys_vendor", "LENOVO");
        write(r, "sys/class/dmi/id/product_name", "ThinkCentre M75q");
        write(r, "sys/class/dmi/id/product_serial", "PC0X7J9H");
        std::fs::create_dir_all(r.join("sys/firmware/efi/efivars")).unwrap();
        std::fs::write(
            r.join(format!("sys/firmware/efi/efivars/SecureBoot-{EFI_GUID}")),
            [7, 0, 0, 0, 1],
        )
        .unwrap();
        std::fs::write(
            r.join(format!("sys/firmware/efi/efivars/SetupMode-{EFI_GUID}")),
            [7, 0, 0, 0, 0],
        )
        .unwrap();
        write(r, "sys/class/tpm/tpm0/tpm_version_major", "2");
        std::fs::create_dir_all(r.join("sys/class/tpm/tpm0/pcr-sha256")).unwrap();
        std::fs::create_dir_all(r.join("sys/class/tpm/tpm0/pcr-sha1")).unwrap();
        write(r, "dev/tpmrm0", "");
        std::fs::create_dir_all(r.join("sys/kernel/iommu_groups/0")).unwrap();
        std::fs::create_dir_all(r.join("sys/kernel/iommu_groups/1")).unwrap();
        write(
            r,
            "proc/cmdline",
            "BOOT_IMAGE=/vmlinuz amd_iommu=on iommu=pt quiet",
        );
        write(
            r,
            "proc/cpuinfo",
            "model name\t: AMD Ryzen 5 PRO 4650GE\nmicrocode\t: 0x8600106\nflags\t\t: fpu vme svm sse\n",
        );
        write(r, "dev/kvm", "");
        std::fs::create_dir_all(r.join("sys/devices/system/edac/mc/mc0")).unwrap();
        write(r, "sys/class/misc/hw_random/rng_current", "tpm-rng-0");
        write(r, "proc/sys/kernel/osrelease", "6.12.1");
        write(
            r,
            "sys/kernel/security/lockdown",
            "none [integrity] confidentiality",
        );
        write(
            r,
            "sys/devices/system/cpu/vulnerabilities/spectre_v2",
            "Mitigation: Retpolines",
        );
        write(
            r,
            "sys/devices/system/cpu/vulnerabilities/meltdown",
            "Not affected",
        );
        write(r, "sys/devices/system/cpu/smt/active", "0");
        write(
            r,
            "sys/kernel/security/lsm",
            "lockdown,capability,yama,apparmor",
        );
        write(r, "sys/module/module/parameters/sig_enforce", "Y");
        write(r, "proc/sys/kernel/randomize_va_space", "2");
        write(r, "sys/block/dm-0/dm/uuid", "CRYPT-LUKS2-abc-cryptroot");
        write(r, "sys/block/dm-0/dm/name", "cryptroot");
        write(r, "sys/block/dm-1/dm/uuid", "LVM-xyz");

        let checks = collect(r);
        assert_eq!(find(&checks, "Boot firmware").verdict, Verdict::Good);
        assert!(find(&checks, "Boot firmware").value.starts_with("UEFI"));
        assert!(find(&checks, "System").note.contains("PC0X7J9H"));
        assert_eq!(find(&checks, "Secure Boot").value, "enabled");
        let tpm = find(&checks, "TPM");
        assert_eq!(tpm.verdict, Verdict::Good);
        assert_eq!(tpm.value, "TPM 2.0 · sha1/sha256");
        assert!(tpm.note.contains("/dev/tpmrm0 is available"));
        assert_eq!(
            find(&checks, "IOMMU").value,
            "2 groups · amd_iommu=on iommu=pt"
        );
        assert_eq!(find(&checks, "Virtualization").value, "AMD-V · /dev/kvm");
        assert_eq!(find(&checks, "ECC memory").verdict, Verdict::Good);
        assert_eq!(find(&checks, "Hardware RNG").value, "tpm-rng-0");
        assert!(find(&checks, "Kernel").note.contains("microcode 0x8600106"));
        assert_eq!(find(&checks, "Lockdown").value, "integrity");
        assert_eq!(
            find(&checks, "CPU mitigations").value,
            "2 checked, 0 vulnerable"
        );
        assert_eq!(find(&checks, "SMT").verdict, Verdict::Good);
        assert_eq!(find(&checks, "Access control").value, "AppArmor");
        assert_eq!(find(&checks, "Module signing").value, "enforced");
        assert_eq!(find(&checks, "ASLR").value, "full");
        assert_eq!(find(&checks, "Disk encryption").value, "LUKS: cryptroot");

        let s = summarize(&checks);
        assert_eq!(s.bad, 0);
        assert_eq!(s.warn, 0);
        assert_eq!(s.info, 2, "System and Kernel rows are informational");
    }

    #[test]
    fn bare_host_fails_loudly() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        write(r, "proc/cpuinfo", "flags\t\t: fpu vme sse\n");
        write(
            r,
            "sys/devices/system/cpu/vulnerabilities/mds",
            "Vulnerable: Clear CPU buffers attempted, no microcode",
        );
        write(
            r,
            "sys/devices/system/cpu/vulnerabilities/l1tf",
            "Mitigation: PTE Inversion",
        );
        write(
            r,
            "sys/class/tpm/tpm0/device/caps",
            "Manufacturer: 0x53544d20\nTCG version: 1.2\n",
        );
        write(
            r,
            "sys/kernel/security/lockdown",
            "[none] integrity confidentiality",
        );

        let checks = collect(r);
        assert_eq!(find(&checks, "Boot firmware").value, "Legacy BIOS · — —");
        assert_eq!(find(&checks, "Secure Boot").verdict, Verdict::Bad);
        let tpm = find(&checks, "TPM");
        assert_eq!(tpm.verdict, Verdict::Warn);
        assert_eq!(tpm.value, "TPM 1.2 (tpm0)");
        assert!(tpm.note.contains("Manufacturer 0x53544d20"));
        assert_eq!(find(&checks, "IOMMU").verdict, Verdict::Warn);
        assert_eq!(find(&checks, "Virtualization").verdict, Verdict::Bad);
        assert_eq!(find(&checks, "CPU mitigations").value, "1 vulnerable of 2");
        assert!(find(&checks, "CPU mitigations").note.contains("mds"));
        assert_eq!(find(&checks, "Lockdown").verdict, Verdict::Warn);
        assert_eq!(find(&checks, "Access control").verdict, Verdict::Warn);
        assert_eq!(find(&checks, "Disk encryption").verdict, Verdict::Warn);
        assert!(summarize(&checks).bad >= 2);
    }

    #[test]
    fn missing_tpm_is_a_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let checks = collect(tmp.path());
        let tpm = find(&checks, "TPM");
        assert_eq!(tpm.verdict, Verdict::Bad);
        assert_eq!(tpm.value, "not present");
    }
}
