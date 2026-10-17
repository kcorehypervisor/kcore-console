//! Which appliance this console is attached to.

use std::path::Path;

use clap::ValueEnum;

/// Marker file written by the appliance image: `kcore-vault` on a backup host.
pub const TYPE_FILE: &str = "/etc/kcore-type";

/// Selects labels, the local API port, and the systemd units on the Diagnostics page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Product {
    /// kcore node: node-agent, controller, and dashboard.
    #[default]
    Hypervisor,
    /// kcore Vault: coordinator, dashboard, and mover.
    Vault,
}

impl Product {
    /// Map the contents of `/etc/kcore-type` to a product.
    pub fn from_type_marker(text: &str) -> Option<Self> {
        let word = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))?
            .to_ascii_lowercase();
        match word.as_str() {
            "kcore-vault" | "vault" | "kcore-backup" | "backup" => Some(Self::Vault),
            "kcore" | "kcore-hypervisor" | "hypervisor" | "kcore-node" | "node" => {
                Some(Self::Hypervisor)
            }
            _ => None,
        }
    }

    /// Read the marker file; `None` when it is missing or unrecognised.
    pub fn from_type_file(path: &Path) -> Option<Self> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| Self::from_type_marker(&t))
    }

    /// Resolve the product: explicit CLI/env choice, else the marker file, else hypervisor.
    pub fn resolve(explicit: Option<Self>, type_file: &Path) -> (Self, ProductSource) {
        if let Some(p) = explicit {
            return (p, ProductSource::Explicit);
        }
        if let Some(p) = Self::from_type_file(type_file) {
            return (p, ProductSource::TypeFile);
        }
        (Self::Hypervisor, ProductSource::Default)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hypervisor => "kcore hypervisor",
            Self::Vault => "kcore vault",
        }
    }

    /// Short word in the console brand line.
    pub fn brand_word(self) -> &'static str {
        match self {
            Self::Hypervisor => "HYPERVISOR",
            Self::Vault => "VAULT",
        }
    }

    /// One-line description under the brand word.
    pub fn tagline(self) -> &'static str {
        match self {
            Self::Hypervisor => "bare-metal hypervisor node",
            Self::Vault => "backup service appliance",
        }
    }

    pub fn api_port(self) -> u16 {
        match self {
            Self::Hypervisor => 9091,
            Self::Vault => 9191,
        }
    }

    pub fn management_port(self) -> &'static str {
        match self {
            Self::Hypervisor => "8443",
            Self::Vault => "9190",
        }
    }

    /// Shown when no management URL was discovered.
    pub fn remote_hint_fallback(self) -> &'static str {
        match self {
            Self::Hypervisor => "kcorectl login https://<node-ip>:8443",
            Self::Vault => "vault dashboard http://127.0.0.1:9190",
        }
    }

    /// Default backup repository directory on a Vault host.
    pub fn vault_repository(self) -> &'static str {
        "/var/lib/kcore-vault"
    }

    /// `(label, systemd unit)` rows for the Diagnostics page.
    pub fn units(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Hypervisor => &[
                ("kcore-node-agent", "kcore-node-agent.service"),
                ("kcore-controller", "kcore-controller.service"),
                ("kcore-dashboard", "kcore-dashboard.service"),
            ],
            Self::Vault => &[
                ("vault-coordinator", "kcore-vault-coordinator.service"),
                ("vault-dashboard", "kcore-vault-dashboard.service"),
                ("vault-mover", "kcore-vault-mover.service"),
            ],
        }
    }

    /// Product notes for the Help page, one paragraph per entry.
    pub fn help_notes(self) -> &'static [&'static str] {
        match self {
            Self::Hypervisor => &[
                "This display is read-only. Administer the node with kcorectl and the management URL from a trusted machine.",
                "Reboot, shutdown, and a root shell are intentionally unavailable here.",
                "Network: IPv4 is never truncated. IPv6 shows the first global address and +N when more exist; the Detail card lists them all. Bridge members show as iface -> br0.",
                "GPU: devices bound to vfio-pci are reserved for VM passthrough. The IOMMU group must be isolated for safe passthrough.",
                "Security: TPM 2.0, Secure Boot, IOMMU, and kernel lockdown are the baseline for a compliant node. Amber rows need attention; red rows block compliance.",
            ],
            Self::Vault => &[
                "This display is read-only. The backup dashboard listens on localhost; administer this host over SSH.",
                "Reboot, shutdown, and a root shell are intentionally unavailable here.",
                "Overview shows the backup repository: its path, the device it is mounted from, and remaining capacity.",
                "Diagnostics lists the vault coordinator, dashboard, and mover services.",
                "Security: TPM 2.0, Secure Boot, and encrypted repository volumes protect backup data at rest.",
            ],
        }
    }

    pub fn help_text(self) -> String {
        self.help_notes().join("\n\n")
    }
}

/// Where the active product came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductSource {
    Explicit,
    TypeFile,
    Default,
}

#[cfg(test)]
mod tests {
    use super::{Product, ProductSource};

    #[test]
    fn products_probe_different_units() {
        assert_eq!(Product::Hypervisor.label(), "kcore hypervisor");
        assert_eq!(Product::Vault.label(), "kcore vault");
        assert_eq!(Product::Hypervisor.brand_word(), "HYPERVISOR");
        assert_eq!(Product::Vault.brand_word(), "VAULT");
        assert_eq!(Product::Hypervisor.api_port(), 9091);
        assert_eq!(Product::Vault.api_port(), 9191);
        assert!(Product::Vault
            .units()
            .iter()
            .any(|(_, unit)| *unit == "kcore-vault-coordinator.service"));
        assert!(Product::Hypervisor
            .units()
            .iter()
            .any(|(_, unit)| *unit == "kcore-node-agent.service"));
    }

    #[test]
    fn type_marker_words() {
        assert_eq!(
            Product::from_type_marker("kcore-vault\n"),
            Some(Product::Vault)
        );
        assert_eq!(
            Product::from_type_marker("  KCORE-VAULT "),
            Some(Product::Vault)
        );
        assert_eq!(
            Product::from_type_marker("# comment\nkcore\n"),
            Some(Product::Hypervisor)
        );
        assert_eq!(Product::from_type_marker("something-else"), None);
        assert_eq!(Product::from_type_marker(""), None);
    }

    #[test]
    fn resolve_prefers_explicit_then_file_then_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("kcore-type");
        assert_eq!(
            Product::resolve(None, &file),
            (Product::Hypervisor, ProductSource::Default)
        );
        std::fs::write(&file, "kcore-vault\n").unwrap();
        assert_eq!(
            Product::resolve(None, &file),
            (Product::Vault, ProductSource::TypeFile)
        );
        assert_eq!(
            Product::resolve(Some(Product::Hypervisor), &file),
            (Product::Hypervisor, ProductSource::Explicit)
        );
    }
}
