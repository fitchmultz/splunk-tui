//! Cluster command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::{info, warn};

use crate::formatters::{ClusterInfoOutput, ClusterPeerOutput, OutputFormat, get_formatter};

pub async fn run(config: splunk_config::Config, detailed: bool, output_format: &str) -> Result<()> {
    info!("Fetching cluster information");

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

    match client.get_cluster_info().await {
        Ok(cluster_info) => {
            // Fetch peers if detailed
            let peers_output = if detailed {
                match client.get_cluster_peers().await {
                    Ok(peers) => Some(peers.into_iter().map(ClusterPeerOutput::from).collect()),
                    Err(e) => {
                        warn!("Could not fetch cluster peers: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let info = ClusterInfoOutput {
                id: cluster_info.id,
                label: cluster_info.label,
                mode: cluster_info.mode,
                manager_uri: cluster_info.manager_uri,
                replication_factor: cluster_info.replication_factor,
                search_factor: cluster_info.search_factor,
                status: cluster_info.status,
                peers: peers_output,
            };

            // Parse output format
            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);

            // Format and print cluster info
            let output = formatter.format_cluster_info(&info, detailed)?;
            print!("{}", output);
        }
        Err(e) => {
            // Not all Splunk instances are clustered
            warn!("Could not fetch cluster info: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    Ok(())
}
