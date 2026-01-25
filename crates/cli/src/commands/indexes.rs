//! Indexes command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use tracing::info;

use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    offset: usize,
    output_format: &str,
) -> Result<()> {
    info!("Listing indexes (count: {}, offset: {})", count, offset);

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let count_u64 =
        u64::try_from(count).context("Invalid --count (value too large for this platform)")?;
    let offset_u64 =
        u64::try_from(offset).context("Invalid --offset (value too large for this platform)")?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset_u64) };

    let indexes = client.list_indexes(Some(count_u64), offset_param).await?;

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;

    // Table output gets pagination footer; machine-readable formats must not.
    if format == OutputFormat::Table {
        let formatter = TableFormatter;
        let pagination = Pagination {
            offset,
            page_size: count,
            total: None, // server-side total is not available in current client response shape
        };
        let output = formatter.format_indexes_paginated(&indexes, detailed, pagination)?;
        print!("{}", output);
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_indexes(&indexes, detailed)?;
    print!("{}", output);

    Ok(())
}
