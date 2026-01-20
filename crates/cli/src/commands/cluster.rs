//! Cluster command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::{info, warn};

use crate::formatters::{ClusterInfoOutput, OutputFormat, get_formatter};

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
            let info = ClusterInfoOutput {
                id: cluster_info.id,
                label: cluster_info.label,
                mode: cluster_info.mode,
                manager_uri: cluster_info.manager_uri,
                replication_factor: cluster_info.replication_factor,
                search_factor: cluster_info.search_factor,
                status: cluster_info.status,
            };

            // Parse output format
            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);

            // Format and print cluster info
            let output = formatter.format_cluster_info(&info)?;
            print!("{}", output);

            if detailed {
                match client.get_cluster_peers().await {
                    Ok(peers) => {
                        println!("\nCluster Peers ({}):\n", peers.len());
                        for peer in peers {
                            println!("  Host: {}:{}", peer.host, peer.port);
                            println!("    ID: {}", peer.id);
                            println!("    Status: {}", peer.status);
                            println!("    State: {}", peer.peer_state);
                            if let Some(label) = peer.label {
                                println!("    Label: {}", label);
                            }
                            if let Some(site) = peer.site {
                                println!("    Site: {}", site);
                            }
                            if peer.is_captain.unwrap_or(false) {
                                println!("    Captain: Yes");
                            }
                            println!();
                        }
                    }
                    Err(e) => {
                        warn!("Could not fetch cluster peers: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            // Not all Splunk instances are clustered
            warn!("Could not fetch cluster info: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    Ok(())
}
