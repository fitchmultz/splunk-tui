//! Keybindings for non-search, non-job screens.
//!
//! Responsibilities:
//! - Define bindings for list/detail screens like indexes, apps, users, and settings.
//!
//! Does NOT handle:
//! - Resolving input events or mutating App state.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

mod apps;
mod audit;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod indexes;
mod inputs;
mod internal_logs;
mod macros;
mod monitoring;
mod saved_searches;
mod search_peers;
mod settings;
mod shc;
mod status;
mod users;
mod workload;

use super::Keybinding;

pub(super) fn bindings() -> Vec<Keybinding> {
    let mut bindings = Vec::new();
    bindings.extend(indexes::bindings());
    bindings.extend(cluster::bindings());
    bindings.extend(audit::bindings());
    bindings.extend(status::bindings());
    bindings.extend(saved_searches::bindings());
    bindings.extend(macros::bindings());
    bindings.extend(internal_logs::bindings());
    bindings.extend(apps::bindings());
    bindings.extend(users::bindings());
    bindings.extend(search_peers::bindings());
    bindings.extend(configs::bindings());
    bindings.extend(dashboards::bindings());
    bindings.extend(datamodels::bindings());
    bindings.extend(inputs::bindings());
    bindings.extend(settings::bindings());
    bindings.extend(monitoring::bindings());
    bindings.extend(workload::bindings());
    bindings.extend(shc::bindings());
    bindings
}
