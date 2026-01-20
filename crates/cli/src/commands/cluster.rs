//! Cluster command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::{info, warn};

pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    _output_format: &str,
) -> Result<()> {
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
            println!("Cluster Information:");
            println!("  ID: {}", cluster_info.id);
            println!("  Mode: {}", cluster_info.mode);
            if let Some(label) = cluster_info.label {
                println!("  Label: {}", label);
            }
            if let Some(replication) = cluster_info.replication_factor {
                println!("  Replication Factor: {}", replication);
            }
            if let Some(search) = cluster_info.search_factor {
                println!("  Search Factor: {}", search);
            }
            if let Some(status) = cluster_info.status {
                println!("  Status: {}", status);
            }
            println!();
        }
        Err(e) => {
            // Not all Splunk instances are clustered
            warn!("Could not fetch cluster info: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    if detailed {
        match client.get_cluster_peers().await {
            Ok(peers) => {
                println!("Cluster Peers ({}):\n", peers.len());
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

    Ok(())
}
