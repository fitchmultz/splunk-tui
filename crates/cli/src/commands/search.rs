//! Search command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    config: splunk_config::Config,
    query: String,
    wait: bool,
    earliest: Option<&str>,
    latest: Option<&str>,
    max_results: usize,
    output_format: &str,
    quiet: bool,
    output_file: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("Executing search: {}", query);

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    info!("Connecting to {}", client.base_url());

    let results = if wait {
        let progress = crate::progress::SearchProgress::new(!quiet, "Waiting for search");

        let mut on_progress = |done_progress: f64| {
            progress.set_fraction(done_progress);
        };

        let results = client
            .search_with_progress(
                &query,
                true,
                earliest,
                latest,
                Some(max_results as u64),
                if quiet { None } else { Some(&mut on_progress) },
            )
            .await?;

        progress.finish();
        results
    } else {
        client
            .search_with_progress(
                &query,
                false,
                earliest,
                latest,
                Some(max_results as u64),
                None,
            )
            .await?
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_search_results(&results)?;

    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    Ok(())
}
