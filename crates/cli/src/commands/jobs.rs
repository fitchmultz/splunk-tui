//! Jobs command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    mut list: bool,
    inspect: Option<String>,
    cancel: Option<String>,
    delete: Option<String>,
    count: usize,
    output_format: &str,
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

    // If inspect, cancel, or delete action is specified, don't list jobs
    if inspect.is_some() || cancel.is_some() || delete.is_some() {
        list = false;
    }

    // Handle inspect action (NEW)
    if let Some(sid) = inspect {
        info!("Inspecting job: {}", sid);
        let job = client.get_job_status(&sid).await?;

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print job details
        let output = formatter.format_job_details(&job)?;
        print!("{}", output);
        return Ok(());
    }

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

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print jobs
        let output = formatter.format_jobs(&jobs)?;
        print!("{}", output);
    }

    Ok(())
}
