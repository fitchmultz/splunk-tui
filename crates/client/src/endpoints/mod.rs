//! REST API endpoint implementations.

mod alerts;
mod auth;
mod cluster;
mod configs;
mod forwarders;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod parsing;
mod request;
pub mod search;
mod search_peers;
mod server;
mod users;

pub use alerts::{get_fired_alert, list_fired_alerts};
pub use auth::login;
pub use cluster::{get_cluster_info, get_cluster_peers};
pub use configs::{get_config_stanza, list_config_files, list_config_stanzas};
pub use forwarders::list_forwarders;
pub use indexes::list_indexes;
pub use inputs::{disable_input, enable_input, list_inputs_by_type};
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use kvstore::get_kvstore_status;
pub use license::{get_license_usage, list_license_pools, list_license_stacks};
pub use logs::get_internal_logs;
pub use lookups::list_lookup_tables;
pub use parsing::check_log_parsing_health;
pub use request::send_request_with_retry;
pub use search::{
    CreateJobOptions, OutputMode, create_job, create_saved_search, delete_saved_search,
    get_job_status, get_results, get_saved_search, list_saved_searches, wait_for_job,
    wait_for_job_with_progress,
};
pub use search_peers::list_search_peers;
pub use server::*;
pub use users::list_users;
