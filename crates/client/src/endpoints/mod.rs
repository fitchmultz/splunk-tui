//! REST API endpoint implementations.

mod alerts;
mod audit;
mod auth;
mod capabilities;
mod cluster;
mod configs;
mod forwarders;
pub mod hec;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod macros;
mod parsing;
mod request;
mod roles;
pub mod search;
mod search_peers;
mod server;
mod users;

pub use alerts::{get_fired_alert, list_fired_alerts};
pub use audit::{get_recent_audit_events, list_audit_events};
pub use auth::login;
pub use capabilities::list_capabilities;
pub use cluster::{
    decommission_peer, get_cluster_info, get_cluster_peers, rebalance_cluster, remove_peers,
    set_maintenance_mode,
};
pub use configs::{get_config_stanza, list_config_files, list_config_stanzas};
pub use forwarders::list_forwarders;
pub use indexes::{create_index, delete_index, list_indexes, modify_index};
pub use inputs::{disable_input, enable_input, list_inputs_by_type};
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use kvstore::{
    create_collection, delete_collection, delete_collection_record, get_kvstore_status,
    insert_collection_record, list_collection_records, list_collections, modify_collection,
};
pub use license::{
    activate_license, create_license_pool, deactivate_license, delete_license_pool,
    get_license_usage, install_license, list_installed_licenses, list_license_pools,
    list_license_stacks, modify_license_pool,
};
pub use logs::get_internal_logs;
pub use lookups::list_lookup_tables;
pub use macros::{
    CreateMacroRequest, UpdateMacroRequest, create_macro, delete_macro, get_macro, list_macros,
    update_macro,
};
pub use parsing::check_log_parsing_health;
pub use request::send_request_with_retry;
pub use roles::{create_role, delete_role, list_roles, modify_role};
pub use search::{
    CreateJobOptions, OutputMode, SavedSearchUpdateParams, create_job, create_saved_search,
    delete_saved_search, get_job_status, get_results, get_saved_search, list_saved_searches,
    update_saved_search, wait_for_job, wait_for_job_with_progress,
};
pub use search_peers::list_search_peers;
pub use server::*;
pub use users::{create_user, delete_user, list_users, modify_user};
