//! REST API endpoint implementations.

mod auth;
mod cluster;
mod indexes;
mod jobs;
mod request;
pub mod search;
mod server;

pub use auth::login;
pub use cluster::{get_cluster_info, get_cluster_peers};
pub use indexes::list_indexes;
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use request::send_request_with_retry;
pub use search::{
    CreateJobOptions, OutputMode, create_job, get_job_status, get_results, wait_for_job,
};
pub use server::get_server_info;
