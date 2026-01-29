//! REST API endpoint implementations.

mod auth;
mod cluster;
mod indexes;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod parsing;
mod request;
pub mod search;
mod server;
mod users;

pub use auth::login;
pub use cluster::{get_cluster_info, get_cluster_peers};
pub use indexes::list_indexes;
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use kvstore::get_kvstore_status;
pub use license::{get_license_usage, list_license_pools, list_license_stacks};
pub use logs::get_internal_logs;
pub use parsing::check_log_parsing_health;
pub use request::send_request_with_retry;
pub use search::{
    CreateJobOptions, OutputMode, create_job, create_saved_search, delete_saved_search,
    get_job_status, get_results, get_saved_search, list_saved_searches, wait_for_job,
    wait_for_job_with_progress,
};
pub use server::*;
pub use users::list_users;
