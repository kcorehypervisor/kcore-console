//! Host inventory: network, disks, metadata, API probe, diagnostics.

pub mod api;
pub mod diagnostics;
pub mod disk;
pub mod format;
pub mod logs;
pub mod network;
pub mod route;
pub mod system;

use self::system::Meta;
use crate::product::Product;

/// Full snapshot for one UI refresh.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub meta: Meta,
    pub nics: Vec<network::Nic>,
    pub disks: Vec<disk::Disk>,
    pub diag: Vec<diagnostics::ServiceLine>,
    pub logs: Vec<logs::LogLine>,
}

/// Collects inventory (may be partially empty on errors).
pub fn load_snapshot(product: Product) -> Snapshot {
    let api = api::probe(product.api_port());
    let nics = network::list_nics();
    let disks = disk::list_disks();
    let diag = diagnostics::kcore_diagnostics(product.units());
    let logs = logs::recent_system_logs(80);
    let meta = system::load_meta(&api, product);
    Snapshot {
        meta,
        nics,
        disks,
        diag,
        logs,
    }
}
