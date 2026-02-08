//! Benchmarks for model deserialization from JSON.
//!
//! Tests parsing of large result sets (1k/10k/100k rows).

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use splunk_client::IndexListResponse;
use splunk_client::models::{ClusterInfo, SearchJobResults};

fn generate_search_results(count: usize) -> String {
    let results: Vec<serde_json::Value> = (0..count)
        .map(|i| {
            serde_json::json!({
                "_time": format!("2024-01-15T10:{:02}:00.000Z", i % 60),
                "_raw": format!("Event {} content here with some data and more text to simulate realistic event sizes", i),
                "sourcetype": "test_sourcetype",
                "index": "main",
                "host": "testhost",
                "source": "/var/log/test.log",
                "field1": format!("value{}", i),
                "field2": i,
                "count": format!("{}", i * 10),
            })
        })
        .collect();

    serde_json::to_string(&serde_json::json!({
        "results": results,
        "preview": false,
        "offset": 0,
        "total": count
    }))
    .unwrap()
}

fn generate_indexes(count: usize) -> String {
    let entries: Vec<serde_json::Value> = (0..count)
        .map(|i| {
            serde_json::json!({
                "name": format!("index_{}", i),
                "content": {
                    "maxTotalDataSizeMB": "1000",
                    "currentDBSizeMB": "500",
                    "totalEventCount": "1000000",
                    "maxWarmDBCount": "300",
                    "maxHotBuckets": "3",
                    "frozenTimePeriodInSecs": "2592000",
                    "coldDBPath": "/opt/splunk/colddb",
                    "homePath": "/opt/splunk/var/lib/splunk",
                    "thawedPath": "/opt/splunk/thaweddb",
                    "coldToFrozenDir": "",
                    "primaryIndex": true
                }
            })
        })
        .collect();

    serde_json::to_string(&serde_json::json!({ "entry": entries })).unwrap()
}

fn bench_search_job_results_1k(c: &mut Criterion) {
    let json = generate_search_results(1_000);
    c.bench_function("search_job_results_1k", |b| {
        b.iter(|| {
            let results: SearchJobResults = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
}

fn bench_search_job_results_10k(c: &mut Criterion) {
    let json = generate_search_results(10_000);
    c.bench_function("search_job_results_10k", |b| {
        b.iter(|| {
            let results: SearchJobResults = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
}

fn bench_search_job_results_50k(c: &mut Criterion) {
    let json = generate_search_results(50_000);
    c.bench_function("search_job_results_50k", |b| {
        b.iter(|| {
            let results: SearchJobResults = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
}

fn bench_index_list_100(c: &mut Criterion) {
    let json = generate_indexes(100);
    c.bench_function("index_list_100", |b| {
        b.iter(|| {
            let results: IndexListResponse = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
}

fn bench_index_list_1k(c: &mut Criterion) {
    let json = generate_indexes(1_000);
    c.bench_function("index_list_1k", |b| {
        b.iter(|| {
            let results: IndexListResponse = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
}

fn bench_cluster_info(c: &mut Criterion) {
    let json = r#"{
        "entry": [{
            "name": "cluster_config",
            "content": {
                "id": "cluster-01",
                "label": "Production Cluster",
                "mode": "peer",
                "manager_uri": "https://cluster-manager:8089",
                "replication_factor": 3,
                "search_factor": 2,
                "status": "enabled",
                "maintenance_mode": false
            }
        }]
    }"#;

    c.bench_function("cluster_info", |b| {
        b.iter(|| {
            // ClusterInfo is extracted from entry content
            let response: serde_json::Value = serde_json::from_str(black_box(json)).unwrap();
            let content = response["entry"][0]["content"].clone();
            let info: ClusterInfo = serde_json::from_value(content).unwrap();
            black_box(info)
        })
    });
}

// Parse 10MB JSON benchmark (target: <100ms)
fn bench_parse_10mb_json(c: &mut Criterion) {
    let json = generate_search_results(50_000); // Approx 10MB
    let json_size_mb = json.len() as f64 / (1024.0 * 1024.0);

    let mut group = c.benchmark_group("memory_usage");
    group.bench_function(format!("parse_{:.1}mb_json", json_size_mb), |b| {
        b.iter(|| {
            let results: SearchJobResults = serde_json::from_str(black_box(&json)).unwrap();
            black_box(results)
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_search_job_results_1k,
    bench_search_job_results_10k,
    bench_search_job_results_50k,
    bench_index_list_100,
    bench_index_list_1k,
    bench_cluster_info,
    bench_parse_10mb_json
);
criterion_main!(benches);
