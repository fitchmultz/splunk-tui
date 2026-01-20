//! Jobs command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::info;

pub async fn run(
    config: splunk_config::Config,
    list: bool,
    cancel: Option<String>,
    delete: Option<String>,
    count: usize,
    _output_format: &str,
) -> Result<()> {
    let auth_strategy = match config.auth.strategy {
        splunk_config::AuthStrategy::SessionToken { username, password } => {
            AuthStrategy::SessionToken { username, password }
        }
        splunk_config::AuthStrategy::ApiToken { token } => AuthStrategy::ApiToken { token },
    };

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    if let Some(sid) = cancel {
        info!("Canceling job: {}", sid);
        client.cancel_job(&sid).await?;
        println!("Job {} canceled.", sid);
        return Ok(());
    }

    if let Some(sid) = delete {
        info!("Deleting job: {}", sid);
        client.delete_job(&sid).await?;
        println!("Job {} deleted.", sid);
        return Ok(());
    }

    if list {
        info!("Listing search jobs");
        let jobs = client.list_jobs(Some(count as u64), None).await?;

        println!("Found {} jobs:\n", jobs.len());

        for job in jobs {
            println!("  SID: {}", job.sid);
            println!("    Done: {}", job.is_done);
            println!("    Progress: {:.1}%", job.done_progress * 100.0);
            println!("    Results: {}", job.result_count);
            println!("    Events: {}", job.event_count);
            println!();
        }
    }

    Ok(())
}
