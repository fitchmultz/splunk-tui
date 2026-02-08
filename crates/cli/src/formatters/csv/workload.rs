//! Workload management CSV formatter.
//!
//! Responsibilities:
//! - Format workload pools and rules as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::{WorkloadPool, WorkloadRule};

/// Format workload pools as CSV.
pub fn format_workload_pools(pools: &[WorkloadPool], detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Header
    let mut headers = vec![
        "name",
        "cpu_weight",
        "mem_weight",
        "default_pool",
        "enabled",
    ];
    if detailed {
        headers.extend(vec![
            "search_concurrency",
            "search_time_range",
            "admission_rules_enabled",
            "cpu_cores",
            "mem_limit",
        ]);
    }
    output.push_str(&build_csv_header(&headers));

    for pool in pools {
        let default_pool = pool
            .default_pool
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("");
        let enabled = pool
            .enabled
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("");
        let cpu_weight = pool.cpu_weight.map(|v| v.to_string()).unwrap_or_default();
        let mem_weight = pool.mem_weight.map(|v| v.to_string()).unwrap_or_default();

        let mut values = vec![
            escape_csv(&pool.name),
            escape_csv(&cpu_weight),
            escape_csv(&mem_weight),
            escape_csv(default_pool),
            escape_csv(enabled),
        ];

        if detailed {
            let concurrency = pool
                .search_concurrency
                .map(|v| v.to_string())
                .unwrap_or_default();
            let admission = pool
                .admission_rules_enabled
                .map(|v| if v { "true" } else { "false" })
                .unwrap_or("");
            let cpu_cores = pool.cpu_cores.map(|v| v.to_string()).unwrap_or_default();
            let mem_limit = pool.mem_limit.map(|v| v.to_string()).unwrap_or_default();
            values.extend(vec![
                escape_csv(&concurrency),
                format_opt_str(pool.search_time_range.as_deref(), ""),
                escape_csv(admission),
                escape_csv(&cpu_cores),
                escape_csv(&mem_limit),
            ]);
        }

        output.push_str(&build_csv_row(&values));
    }

    Ok(output)
}

/// Format workload rules as CSV.
pub fn format_workload_rules(rules: &[WorkloadRule], detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Header
    let mut headers = vec!["name", "workload_pool", "predicate", "enabled"];
    if detailed {
        headers.extend(vec![
            "user",
            "app",
            "search_type",
            "search_time_range",
            "order",
        ]);
    }
    output.push_str(&build_csv_header(&headers));

    for rule in rules {
        let enabled = rule
            .enabled
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("");

        let mut values = vec![
            escape_csv(&rule.name),
            format_opt_str(rule.workload_pool.as_deref(), ""),
            format_opt_str(rule.predicate.as_deref(), ""),
            escape_csv(enabled),
        ];

        if detailed {
            let order = rule.order.map(|v| v.to_string()).unwrap_or_default();
            values.extend(vec![
                format_opt_str(rule.user.as_deref(), ""),
                format_opt_str(rule.app.as_deref(), ""),
                format_opt_str(
                    rule.search_type.as_ref().map(|t| t.to_string()).as_deref(),
                    "",
                ),
                format_opt_str(rule.search_time_range.as_deref(), ""),
                escape_csv(&order),
            ]);
        }

        output.push_str(&build_csv_row(&values));
    }

    Ok(output)
}
