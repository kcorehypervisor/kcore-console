//! Backup repository status for a kcore Vault host.

use std::path::Path;

use super::format::format_bytes;
use crate::product::Product;

/// Where the backup repository lives and how full it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub path: String,
    /// The directory exists.
    pub present: bool,
    /// Device from the mount table, e.g. `/dev/mapper/vault`.
    pub source: String,
    pub fstype: String,
    pub mountpoint: String,
    pub total: u64,
    pub used: u64,
    pub avail: u64,
}

impl Default for Repository {
    fn default() -> Self {
        Self {
            path: Product::Vault.vault_repository().to_string(),
            present: false,
            source: "—".into(),
            fstype: "—".into(),
            mountpoint: "—".into(),
            total: 0,
            used: 0,
            avail: 0,
        }
    }
}

impl Repository {
    pub fn total_text(&self) -> String {
        bytes_or_dash(self.total)
    }

    pub fn used_text(&self) -> String {
        bytes_or_dash(self.used)
    }

    pub fn avail_text(&self) -> String {
        bytes_or_dash(self.avail)
    }

    /// 0..=100, or `None` when the size is unknown.
    pub fn used_pct(&self) -> Option<u8> {
        if self.total == 0 {
            None
        } else {
            Some(((self.used.saturating_mul(100)) / self.total).min(100) as u8)
        }
    }
}

fn bytes_or_dash(n: u64) -> String {
    if n == 0 {
        "—".into()
    } else {
        format_bytes(n)
    }
}

/// A mounted filesystem that covers `path` (`/` always qualifies; the longest wins).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveringMount {
    pub source: String,
    pub mountpoint: String,
    pub fstype: String,
}

/// `/proc/mounts` escapes spaces as `\040`.
fn unescape_mount(s: &str) -> String {
    s.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

/// Longest mount point that contains `path`.
pub fn mount_covering(mounts: &str, path: &str) -> Option<CoveringMount> {
    let mut best: Option<CoveringMount> = None;
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let Some(source) = parts.next() else { continue };
        let Some(point) = parts.next() else { continue };
        let Some(fstype) = parts.next() else { continue };
        let point = unescape_mount(point);
        let covers = path == point || path.starts_with(&format!("{point}/")) || point == "/";
        // A mount at "/" covers everything, but only when the path is absolute.
        let covers = covers && (point != "/" || path.starts_with('/'));
        if !covers {
            continue;
        }
        let longer = best
            .as_ref()
            .map(|b| point.len() > b.mountpoint.len())
            .unwrap_or(true);
        if longer {
            best = Some(CoveringMount {
                source: source.to_string(),
                mountpoint: point,
                fstype: fstype.to_string(),
            });
        }
    }
    best
}

#[cfg(unix)]
fn usage(path: &Path) -> Option<(u64, u64, u64)> {
    let bytes = std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str());
    let c = std::ffi::CString::new(bytes).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let block = st.f_frsize;
    let total = st.f_blocks.saturating_mul(block);
    let avail = st.f_bavail.saturating_mul(block);
    let free = st.f_bfree.saturating_mul(block);
    Some((total, total.saturating_sub(free), avail))
}

#[cfg(not(unix))]
fn usage(_path: &Path) -> Option<(u64, u64, u64)> {
    None
}

/// Repository status for `path`, using `mounts` (`/proc/mounts` text) for the device.
pub fn repository_from(path: &Path, mounts: &str) -> Repository {
    let covering = mount_covering(mounts, &path.to_string_lossy());
    let present = path.is_dir();
    let (total, used, avail) = if present {
        usage(path).unwrap_or((0, 0, 0))
    } else {
        (0, 0, 0)
    };
    Repository {
        path: path.display().to_string(),
        present,
        source: covering
            .as_ref()
            .map(|m| m.source.clone())
            .unwrap_or_else(|| "—".into()),
        fstype: covering
            .as_ref()
            .map(|m| m.fstype.clone())
            .unwrap_or_else(|| "—".into()),
        mountpoint: covering
            .as_ref()
            .map(|m| m.mountpoint.clone())
            .unwrap_or_else(|| "—".into()),
        total,
        used,
        avail,
    }
}

/// Status of the vault repository on this host.
pub fn repository_status() -> Repository {
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    repository_from(Path::new(Product::Vault.vault_repository()), &mounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "\
/dev/nvme0n1p2 / ext4 rw 0 0
/dev/nvme0n1p1 /boot vfat rw 0 0
/dev/mapper/vault /var/lib/kcore-vault xfs rw 0 0
tmpfs /run tmpfs rw 0 0
";

    #[test]
    fn longest_mount_wins() {
        let m = mount_covering(MOUNTS, "/var/lib/kcore-vault").unwrap();
        assert_eq!(m.source, "/dev/mapper/vault");
        assert_eq!(m.fstype, "xfs");
        assert_eq!(m.mountpoint, "/var/lib/kcore-vault");
        let root = mount_covering(MOUNTS, "/etc").unwrap();
        assert_eq!(root.mountpoint, "/");
        assert!(mount_covering("", "relative").is_none());
    }

    #[test]
    fn unescapes_spaces_in_mount_points() {
        let m = mount_covering(
            "tmpfs /media/backup\\040disk ext4 rw 0 0\n",
            "/media/backup disk/repo",
        )
        .unwrap();
        assert_eq!(m.mountpoint, "/media/backup disk");
    }

    #[test]
    fn missing_directory_reports_the_mount_but_no_size() {
        let repo = repository_from(Path::new("/var/lib/kcore-vault-does-not-exist"), MOUNTS);
        assert!(!repo.present);
        // No dedicated mount, so the root filesystem is the covering mount, and there is no size.
        assert_eq!(repo.source, "/dev/nvme0n1p2");
        assert_eq!(repo.used_pct(), None);
    }

    #[test]
    fn real_directory_has_a_size() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository_from(dir.path(), "/dev/root / ext4 rw 0 0\n");
        assert!(repo.present);
        assert!(repo.total > 0, "statvfs should report the filesystem size");
        assert!(repo.used_pct().is_some());
    }
}
