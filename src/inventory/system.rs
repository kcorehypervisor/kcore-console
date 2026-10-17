//! Hostname, uptime, version, management URL, and optional kcore cluster metadata.

use chrono::Local;
use std::time::Duration;

use super::api::ApiStatus;
use crate::product::Product;

/// Build-time optional git short hash.
pub const GIT_SHORT: &str = match option_env!("KCORE_GIT_REV") {
    Some(s) => s,
    None => "",
};

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct Meta {
    pub kind: Product,
    pub product: String,
    pub version: String,
    pub build_id: String,
    pub hostname: String,
    pub uptime: Duration,
    pub uptime_str: String,
    pub local_time: String,
    pub management_url: String,
    pub api_endpoint: String,
    pub api_status: ApiStatus,
    pub cluster_name: String,
    pub node_role: String,
    /// Data-center id from the node agent or controller config, such as `DC1`.
    pub dc_id: String,
    pub health: Health,
    pub local_login: &'static str,
    pub remote_hint: String,
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            kind: Product::Hypervisor,
            product: Product::Hypervisor.label().to_string(),
            version: PKG_VERSION.to_string(),
            build_id: if GIT_SHORT.is_empty() {
                "—".to_string()
            } else {
                GIT_SHORT.to_string()
            },
            hostname: "loading".to_string(),
            uptime: Duration::from_secs(0),
            uptime_str: "—".to_string(),
            local_time: "—".to_string(),
            management_url: "—".to_string(),
            api_endpoint: "127.0.0.1:9091".to_string(),
            api_status: ApiStatus::Unavailable,
            cluster_name: "—".to_string(),
            node_role: "—".to_string(),
            dc_id: "—".to_string(),
            health: Health::Unknown,
            local_login: "disabled",
            remote_hint: "kcorectl login https://<node-ip>:8443".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Ok,
    Degraded,
    Unknown,
    Critical,
}

impl std::fmt::Display for Health {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Health::Ok => "OK",
            Health::Degraded => "Degraded",
            Health::Unknown => "Unknown",
            Health::Critical => "Critical",
        };
        write!(f, "{s}")
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

/// `clusterName = "kcore-prod";` assignments, in file order, without duplicates.
pub(crate) fn cluster_names_from_nix(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let code = line.split('#').next().unwrap_or("");
        let Some(rest) = code.trim().strip_prefix("clusterName") else {
            continue;
        };
        let rest = rest.trim();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let name = rest
            .trim()
            .trim_end_matches(';')
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if !name.is_empty() && !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }
    names
}

fn yaml_value(path: &str, key: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| yaml_scalar(&text, key))
}

/// First `key: "value"` scalar in a small YAML config. Quoted or bare.
pub(crate) fn yaml_scalar(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix(&prefix) else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn file_first_line(p: &str) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .filter(|s| !s.is_empty())
}

fn run_ip_json() -> Option<serde_json::Value> {
    let out = std::process::Command::new("ip")
        .args(["-j", "address", "show"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn parse_primary_ipv4(v: &serde_json::Value, default_if: &str) -> Option<String> {
    let arr = v.as_array()?;
    for iface in arr {
        if iface.get("ifname")?.as_str()? != default_if {
            continue;
        }
        for a in iface.get("addr_info")?.as_array()? {
            if a.get("family")?.as_str()? != "inet" {
                continue;
            }
            if a.get("scope")?.as_str()? == "global" {
                return a.get("local")?.as_str().map(String::from);
            }
        }
    }
    None
}

/// Primary IPv4 for management URL, best-effort.
fn primary_ipv4_for_mgmt() -> Option<String> {
    if let Some(ip) = env("KCORE_MANAGEMENT_IP") {
        return Some(ip);
    }
    let ifname = super::route::read_default_ifname()?;
    let j = run_ip_json()?;
    parse_primary_ipv4(&j, &ifname)
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    let days = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    if days > 0 {
        format!("{days}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m {sec}s", sec = s % 60)
    }
}

/// Snapshot metadata. Tolerates missing /proc, tools, and files.
pub fn load_meta(api: &ApiStatus, product: Product) -> Meta {
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let mut uptime = Duration::from_secs(0);
    if let Ok(c) = std::fs::read_to_string("/proc/uptime") {
        if let Some(f) = c.split_whitespace().next() {
            if let Ok(s) = f.parse::<f64>() {
                uptime = Duration::from_secs_f64(s);
            }
        }
    }
    let uptime_str = format_duration(uptime);

    let kcore_version = std::fs::read_to_string("/run/kcore/version")
        .or_else(|_| std::fs::read_to_string("/etc/kcore/version"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let version = if !kcore_version.is_empty() {
        kcore_version
    } else {
        PKG_VERSION.to_string()
    };

    let build_id = if !GIT_SHORT.is_empty() {
        GIT_SHORT.to_string()
    } else {
        "—".to_string()
    };

    let cluster_name = env("KCORE_CLUSTER_NAME")
        .or_else(|| file_first_line("/etc/kcore/cluster_name"))
        .or_else(|| {
            std::fs::read_to_string("/etc/nixos/kcore-kubernetes.nix")
                .ok()
                .and_then(|text| {
                    let names = cluster_names_from_nix(&text);
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.join(", "))
                    }
                })
        })
        .unwrap_or_else(|| "—".to_string());

    let node_role = env("KCORE_NODE_ROLE")
        .or_else(|| file_first_line("/etc/kcore/node_role"))
        .unwrap_or_else(|| "—".to_string());

    let dc_id = env("KCORE_DC_ID")
        .or_else(|| file_first_line("/etc/kcore/dc_id"))
        .or_else(|| yaml_value("/etc/kcore/node-agent.yaml", "dcId"))
        .or_else(|| yaml_value("/etc/kcore/controller.yaml", "dcId"))
        .unwrap_or_else(|| "—".to_string());

    let management_url = if let Some(u) = env("KCORE_MANAGEMENT_URL") {
        u
    } else if product == Product::Vault {
        "http://127.0.0.1:9190".to_string()
    } else if let Some(ip) = primary_ipv4_for_mgmt() {
        let port = env("KCORE_MANAGEMENT_PORT").unwrap_or_else(|| product.management_port().into());
        format!("https://{ip}:{port}")
    } else {
        "—".to_string()
    };

    let health = match api {
        ApiStatus::Reachable { healthy } if *healthy => Health::Ok,
        ApiStatus::Reachable { .. } => Health::Degraded,
        _ => Health::Unknown,
    };
    let api_endpoint = format!(
        "127.0.0.1:{}",
        env("KCORE_API_PORT").unwrap_or_else(|| product.api_port().to_string())
    );

    let local_time = Local::now().format("%Y-%m-%d %H:%M:%S (local)").to_string();

    let remote = if let Some(hint) = env("KCORE_REMOTE_HINT") {
        hint
    } else if product == Product::Vault {
        if management_url == "—" {
            product.remote_hint_fallback().to_string()
        } else {
            format!("vault dashboard {management_url}")
        }
    } else if management_url == "—" {
        product.remote_hint_fallback().to_string()
    } else {
        let with_scheme =
            if management_url.starts_with("https://") || management_url.starts_with("http://") {
                management_url.clone()
            } else {
                format!("https://{}", management_url.trim_start_matches('/'))
            };
        format!("kcorectl login {with_scheme}")
    };

    Meta {
        kind: product,
        product: product.label().to_string(),
        version,
        build_id,
        hostname,
        uptime,
        uptime_str,
        local_time,
        management_url,
        api_endpoint,
        api_status: api.clone(),
        cluster_name,
        node_role,
        dc_id,
        health,
        local_login: "disabled",
        remote_hint: remote,
    }
}

#[cfg(test)]
mod tests {
    use super::{cluster_names_from_nix, yaml_scalar};

    #[test]
    fn nix_cluster_names_keep_file_order() {
        let text = r#"
          clusterName = "kcore-prod";
          # clusterName = "ignored";
          additionalClusters."kcore-b" = {
            clusterName = "kcore-b";
          };
          clusterName = "kcore-prod";
        "#;
        assert_eq!(
            cluster_names_from_nix(text),
            vec!["kcore-prod".to_string(), "kcore-b".to_string()]
        );
    }

    #[test]
    fn yaml_dc_id_reads_a_quoted_scalar() {
        let text = "replication:\n  dcId: \"DC1\"\n";
        assert_eq!(yaml_scalar(text, "dcId").as_deref(), Some("DC1"));
    }
}
