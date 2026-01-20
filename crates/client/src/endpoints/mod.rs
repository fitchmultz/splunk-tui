//! REST API endpoint implementations.

mod auth;
mod cluster;
mod indexes;
mod jobs;
mod kvstore;
mod license;
mod parsing;
mod request;
pub mod search;
mod server;

pub use auth::login;
pub use cluster::{get_cluster_info, get_cluster_peers};
pub use indexes::list_indexes;
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use kvstore::get_kvstore_status;
pub use license::get_license_usage;
pub use parsing::check_log_parsing_health;
pub use request::send_request_with_retry;
pub use search::{
    CreateJobOptions, OutputMode, create_job, get_job_status, get_results, wait_for_job,
};
pub use server::{get_health, get_server_info};
