//! Benchmarks for CLI output formatters.
//!
//! Tests CSV, table, and JSON formatting performance using public APIs.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

// Use splunk_client directly for model types
use splunk_client::models::LogLevel;

// Simple CSV formatter for benchmarking
fn format_logs_csv(logs: &[TestLogEntry]) -> String {
    let mut output = String::new();
    output.push_str("time,component,level,message\n");
    for log in logs {
        output.push_str(&format!(
            "{},{},{},{}\n",
            log.time, log.component, log.level, log.message
        ));
    }
    output
}

// Simple JSON formatter for benchmarking
fn format_logs_json(logs: &[TestLogEntry]) -> String {
    serde_json::to_string_pretty(logs).unwrap_or_default()
}

// Simple table formatter for benchmarking
fn format_logs_table(logs: &[TestLogEntry]) -> String {
    let mut output = String::new();
    output.push_str("TIME                | COMPONENT    | LEVEL | MESSAGE\n");
    output.push_str("--------------------+--------------+-------+--------\n");
    for log in logs {
        output.push_str(&format!(
            "{:<19} | {:<12} | {:<5} | {}\n",
            &log.time[..19.min(log.time.len())],
            log.component,
            format!("{:?}", log.level),
            &log.message[..50.min(log.message.len())]
        ));
    }
    output
}

#[derive(Debug, Clone, serde::Serialize)]
struct TestLogEntry {
    time: String,
    component: String,
    level: LogLevel,
    message: String,
}

fn generate_log_entries(count: usize) -> Vec<TestLogEntry> {
    (0..count)
        .map(|i| TestLogEntry {
            time: format!("2024-01-15 10:{:02}:00", i % 60),
            component: format!("component_{}", i % 10),
            level: LogLevel::Info,
            message: format!(
                "Log message number {} with some additional content for realistic sizing",
                i
            ),
        })
        .collect()
}

fn generate_search_results(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "_time": format!("2024-01-15T10:{:02}:00.000Z", i % 60),
                "_raw": format!("Raw event data {} with more content here", i),
                "field1": format!("value{}", i),
                "field2": i,
                "field3": i % 2 == 0,
            })
        })
        .collect()
}

fn generate_wide_results(count: usize, columns: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "_time".to_string(),
                serde_json::json!(format!("2024-01-15T10:{:02}:00.000Z", i % 60)),
            );
            for col in 0..columns {
                obj.insert(
                    format!("column_{}", col),
                    serde_json::json!(format!("value_{}_{}", i, col)),
                );
            }
            serde_json::Value::Object(obj)
        })
        .collect()
}

// CSV formatting benchmarks
fn bench_csv_format_100(c: &mut Criterion) {
    let entries = generate_log_entries(100);
    c.bench_function("csv_logs_100", |b| {
        b.iter(|| black_box(format_logs_csv(black_box(&entries))))
    });
}

fn bench_csv_format_1k(c: &mut Criterion) {
    let entries = generate_log_entries(1_000);
    c.bench_function("csv_logs_1k", |b| {
        b.iter(|| black_box(format_logs_csv(black_box(&entries))))
    });
}

// JSON formatting benchmarks
fn bench_json_format_100(c: &mut Criterion) {
    let entries = generate_log_entries(100);
    c.bench_function("json_logs_100", |b| {
        b.iter(|| black_box(format_logs_json(black_box(&entries))))
    });
}

fn bench_json_format_1k(c: &mut Criterion) {
    let entries = generate_log_entries(1_000);
    c.bench_function("json_logs_1k", |b| {
        b.iter(|| black_box(format_logs_json(black_box(&entries))))
    });
}

fn bench_json_format_10k(c: &mut Criterion) {
    let entries = generate_log_entries(10_000);
    c.bench_function("json_logs_10k", |b| {
        b.iter(|| black_box(format_logs_json(black_box(&entries))))
    });
}

// Table formatting benchmarks
fn bench_table_format_100(c: &mut Criterion) {
    let entries = generate_log_entries(100);
    c.bench_function("table_logs_100", |b| {
        b.iter(|| black_box(format_logs_table(black_box(&entries))))
    });
}

fn bench_table_format_1k(c: &mut Criterion) {
    let entries = generate_log_entries(1_000);
    c.bench_function("table_logs_1k", |b| {
        b.iter(|| black_box(format_logs_table(black_box(&entries))))
    });
}

// CSV-like search results formatting (using serde_json::Value)
fn format_search_results_csv(results: &[serde_json::Value]) -> String {
    let mut output = String::new();
    output.push_str("_time,_raw,field1,field2,field3\n");
    for result in results {
        let time = result.get("_time").and_then(|v| v.as_str()).unwrap_or("");
        let raw = result.get("_raw").and_then(|v| v.as_str()).unwrap_or("");
        let field1 = result.get("field1").and_then(|v| v.as_str()).unwrap_or("");
        let field2 = result
            .get("field2")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let field3 = result
            .get("field3")
            .and_then(|v| v.as_bool())
            .map(|v| v.to_string())
            .unwrap_or_default();
        output.push_str(&format!(
            "{},{},{},{},{}\n",
            time, raw, field1, field2, field3
        ));
    }
    output
}

fn bench_csv_search_results_1k(c: &mut Criterion) {
    let results = generate_search_results(1_000);
    c.bench_function("csv_search_results_1k", |b| {
        b.iter(|| black_box(format_search_results_csv(black_box(&results))))
    });
}

// Wide dataset CSV formatting
fn collect_csv_headers(rows: &[serde_json::Value]) -> Vec<String> {
    let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for row in rows {
        if let Some(obj) = row.as_object() {
            for k in obj.keys() {
                keys.insert(k.clone());
            }
        }
    }
    keys.into_iter().collect()
}

fn format_wide_csv(results: &[serde_json::Value]) -> String {
    let mut output = String::new();
    let headers = collect_csv_headers(results);

    // Write headers
    output.push_str(&headers.join(","));
    output.push('\n');

    // Write rows
    for row in results {
        let values: Vec<String> = headers
            .iter()
            .map(|h| {
                row.get(h)
                    .map(|v| {
                        v.as_str()
                            .map(String::from)
                            .unwrap_or_else(|| v.to_string())
                    })
                    .unwrap_or_default()
            })
            .collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }

    output
}

fn bench_csv_wide_dataset(c: &mut Criterion) {
    // 1000 rows x 50 columns
    let results = generate_wide_results(1_000, 50);
    c.bench_function("csv_wide_1000x50", |b| {
        b.iter(|| black_box(format_wide_csv(black_box(&results))))
    });
}

criterion_group!(
    benches,
    bench_csv_format_100,
    bench_csv_format_1k,
    bench_csv_search_results_1k,
    bench_csv_wide_dataset,
    bench_table_format_100,
    bench_table_format_1k,
    bench_json_format_100,
    bench_json_format_1k,
    bench_json_format_10k
);
criterion_main!(benches);
