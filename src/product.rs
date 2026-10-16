//! Which appliance this console is attached to.

use clap::ValueEnum;

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
    pub fn label(self) -> &'static str {
        match self {
            Self::Hypervisor => "kcore hypervisor",
            Self::Vault => "kcore vault",
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

    pub fn help_text(self) -> &'static str {
        match self {
            Self::Hypervisor => {
                "KCORE hypervisor appliance — local display is read-only.\
                \nUse the management URL and kcorectl from a trusted machine.\
                \nReboot, shutdown, and root shell are intentionally unavailable here.\
                \n\nTabs: Overview · Network · Storage · Diagnostics · Help\
                \nThe active tab is highlighted. Arrow keys and Tab switch pages.\
                \n\nNetwork: tables adapt to console width. IPv4 is never truncated.\
                \nIPv6 shows the first global address (else link-local) and +N when more\
                \nexist; the full list is on the Detail line under the table.\
                \nBridge members show as \"iface -> br0\" (or a master column when wide).\
                \n\nOverview: node status above, host system log below.\
                \n\nSecurity: protect boot (UEFI, GRUB), mask extra getty, prefer SSH.\n"
            }
            Self::Vault => {
                "KCORE vault appliance — local display is read-only.\
                \nThe dashboard listens on localhost. Administer this host over SSH.\
                \nReboot, shutdown, and root shell are intentionally unavailable here.\
                \n\nTabs: Overview · Network · Storage · Diagnostics · Help\
                \nThe active tab is highlighted. Arrow keys and Tab switch pages.\
                \n\nDiagnostics lists the vault coordinator, dashboard, and mover.\
                \nStorage is the repository disk, mounted at /var/lib/kcore-vault when present.\
                \n\nOverview: host status above, system log below.\
                \n\nSecurity: protect boot (UEFI), mask extra getty, prefer SSH.\n"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Product;

    #[test]
    fn products_probe_different_units() {
        assert_eq!(Product::Hypervisor.label(), "kcore hypervisor");
        assert_eq!(Product::Vault.label(), "kcore vault");
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
}
