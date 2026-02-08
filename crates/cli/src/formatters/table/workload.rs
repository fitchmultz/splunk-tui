//! Workload management table formatter.
//!
//! Responsibilities:
//! - Format workload pools and rules as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in imp.rs).

use anyhow::Result;
use splunk_client::{WorkloadPool, WorkloadRule};

/// Format workload pools as a tab-separated table.
pub fn format_workload_pools(pools: &[WorkloadPool], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if pools.is_empty() {
        return Ok("No workload pools found.".to_string());
    }

    // Header
    if detailed {
        output.push_str(
            "Name\tCPU Weight\tMem Weight\tDefault\tEnabled\tConcurrency\tTime Range\tAdmission Rules\tCPU Cores\tMem Limit\n",
        );
    } else {
        output.push_str("Name\tCPU Weight\tMem Weight\tDefault\tEnabled\n");
    }

    for pool in pools {
        let name = pool.name.clone();
        let cpu_weight = pool
            .cpu_weight
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let mem_weight = pool
            .mem_weight
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let default = pool
            .default_pool
            .map(|v| if v { "Yes" } else { "No" })
            .unwrap_or("N/A");
        let enabled = pool
            .enabled
            .map(|v| if v { "Yes" } else { "No" })
            .unwrap_or("N/A");

        if detailed {
            let concurrency = pool
                .search_concurrency
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let time_range = pool.search_time_range.as_deref().unwrap_or("N/A");
            let admission = pool
                .admission_rules_enabled
                .map(|v| if v { "Yes" } else { "No" })
                .unwrap_or("N/A");
            let cpu_cores = pool
                .cpu_cores
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let mem_limit = pool
                .mem_limit
                .map(|v| format!("{} MB", v))
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                name,
                cpu_weight,
                mem_weight,
                default,
                enabled,
                concurrency,
                time_range,
                admission,
                cpu_cores,
                mem_limit
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                name, cpu_weight, mem_weight, default, enabled
            ));
        }
    }

    Ok(output)
}

/// Format workload rules as a tab-separated table.
pub fn format_workload_rules(rules: &[WorkloadRule], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if rules.is_empty() {
        return Ok("No workload rules found.".to_string());
    }

    // Header
    if detailed {
        output.push_str(
            "Name\tPredicate\tWorkload Pool\tUser\tApp\tSearch Type\tTime Range\tEnabled\tOrder\n",
        );
    } else {
        output.push_str("Name\tWorkload Pool\tPredicate\tEnabled\n");
    }

    for rule in rules {
        let name = rule.name.clone();
        let predicate = rule.predicate.as_deref().unwrap_or("N/A");
        let pool = rule.workload_pool.as_deref().unwrap_or("N/A");
        let enabled = rule
            .enabled
            .map(|v| if v { "Yes" } else { "No" })
            .unwrap_or("N/A");

        if detailed {
            let user = rule.user.as_deref().unwrap_or("N/A");
            let app = rule.app.as_deref().unwrap_or("N/A");
            let search_type = rule
                .search_type
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let time_range = rule.search_time_range.as_deref().unwrap_or("N/A");
            let order = rule
                .order
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                name, predicate, pool, user, app, search_type, time_range, enabled, order
            ));
        } else {
            output.push_str(&format!("{}\t{}\t{}\t{}\n", name, pool, predicate, enabled));
        }
    }

    Ok(output)
}
