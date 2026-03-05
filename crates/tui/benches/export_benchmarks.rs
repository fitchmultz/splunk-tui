//! Benchmarks for export functionality.
//!
//! Tests CSV, JSON, and NDJSON export performance.

use criterion::{Criterion, criterion_group, criterion_main};
use splunk_tui::action::ExportFormat;
use splunk_tui::export::export_results;

fn generate_results(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "_time": format!("2024-01-15T10:30:{:02}.000Z", i % 60),
                "_raw": format!("Event {} content with more data here to simulate realistic event sizes", i),
                "field1": format!("value{}", i),
                "field2": i,
                "field3": i % 2 == 0,
                "field4": format!("additional_field_{}", i),
            })
        })
        .collect()
}

fn bench_export_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_json");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 1_000, 10_000] {
        group.bench_function(format!("json_{}", size), |b| {
            let results = generate_results(size);
            b.to_async(&runtime).iter(|| async {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("test.json");
                export_results(&results, &path, ExportFormat::Json)
                    .await
                    .unwrap();
            })
        });
    }

    group.finish();
}

fn bench_export_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_csv");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 1_000, 10_000] {
        group.bench_function(format!("csv_{}", size), |b| {
            let results = generate_results(size);
            b.to_async(&runtime).iter(|| async {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("test.csv");
                export_results(&results, &path, ExportFormat::Csv)
                    .await
                    .unwrap();
            })
        });
    }

    group.finish();
}

fn bench_export_ndjson(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_ndjson");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 1_000, 10_000] {
        group.bench_function(format!("ndjson_{}", size), |b| {
            let results = generate_results(size);
            b.to_async(&runtime).iter(|| async {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("test.ndjson");
                export_results(&results, &path, ExportFormat::Ndjson)
                    .await
                    .unwrap();
            })
        });
    }

    group.finish();
}

// Benchmark wide datasets (many columns) for CSV
fn generate_wide_results(count: usize, columns: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "_time".to_string(),
                serde_json::json!(format!("2024-01-15T10:30:{:02}.000Z", i % 60)),
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

fn bench_export_csv_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_csv_wide");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Test 1000 rows x 50 columns
    group.bench_function("csv_1000x50", |b| {
        let results = generate_wide_results(1_000, 50);
        b.to_async(&runtime).iter(|| async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("test.csv");
            export_results(&results, &path, ExportFormat::Csv)
                .await
                .unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_export_json,
    bench_export_csv,
    bench_export_ndjson,
    bench_export_csv_wide
);
criterion_main!(benches);
