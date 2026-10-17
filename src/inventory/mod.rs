//! Host inventory: network, disks, metadata, API probe, diagnostics.

pub mod api;
pub mod diagnostics;
pub mod disk;
pub mod format;
pub mod gpu;
pub mod logs;
pub mod network;
pub mod route;
pub mod security;
pub mod system;
pub mod vault;

use self::system::Meta;
use crate::product::Product;

/// Full snapshot for one UI refresh.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub meta: Meta,
    pub nics: Vec<network::Nic>,
    pub disks: Vec<disk::Disk>,
    pub gpus: Vec<gpu::Gpu>,
    pub security: Vec<security::Check>,
    pub diag: Vec<diagnostics::ServiceLine>,
    pub logs: Vec<logs::LogLine>,
    pub repository: vault::Repository,
}

/// Collects inventory (may be partially empty on errors).
pub fn load_snapshot(product: Product) -> Snapshot {
    let api = api::probe(product.api_port());
    let nics = network::list_nics();
    let disks = disk::list_disks();
    let gpus = gpu::list_gpus();
    let security = security::security_checks();
    let diag = diagnostics::kcore_diagnostics(product.units());
    let logs = logs::recent_system_logs(200);
    let repository = vault::repository_status();
    let meta = system::load_meta(&api, product);
    Snapshot {
        meta,
        nics,
        disks,
        gpus,
        security,
        diag,
        logs,
        repository,
    }
}
