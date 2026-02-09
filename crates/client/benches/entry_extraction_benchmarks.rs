//! Benchmarks for entry content extraction and name merging.
//!
//! Tests `extract_entry_content` pattern and `attach_entry_name` functions.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

// Simulated version of extract_entry_content for benchmarking
fn extract_entry_content(resp: &serde_json::Value) -> Option<&serde_json::Value> {
    resp.get("entry")
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .and_then(|entry| entry.get("content"))
}

fn generate_entry_response(count: usize) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = (0..count)
        .map(|i| {
            serde_json::json!({
                "name": format!("entry_{}", i),
                "content": {
                    "id": format!("id_{}", i),
                    "field1": "value1",
                    "field2": 12345,
                    "field3": true,
                    "nested": {
                        "a": 1,
                        "b": 2
                    }
                },
                "acl": {
                    "owner": "admin",
                    "app": "search",
                    "sharing": "app"
                }
            })
        })
        .collect();

    serde_json::json!({ "entry": entries })
}

fn bench_extract_entry_content_single(c: &mut Criterion) {
    let response = generate_entry_response(1);
    c.bench_function("extract_entry_single", |b| {
        b.iter(|| black_box(extract_entry_content(black_box(&response))))
    });
}

fn bench_extract_entry_content_10(c: &mut Criterion) {
    let response = generate_entry_response(10);
    c.bench_function("extract_entry_10", |b| {
        b.iter(|| black_box(extract_entry_content(black_box(&response))))
    });
}

fn bench_extract_entry_content_100(c: &mut Criterion) {
    let response = generate_entry_response(100);
    c.bench_function("extract_entry_100", |b| {
        b.iter(|| black_box(extract_entry_content(black_box(&response))))
    });
}

fn bench_extract_entry_content_1k(c: &mut Criterion) {
    let response = generate_entry_response(1_000);
    c.bench_function("extract_entry_1k", |b| {
        b.iter(|| black_box(extract_entry_content(black_box(&response))))
    });
}

// Benchmark name merging (attach_entry_name pattern)
trait HasName {
    fn set_name(&mut self, name: String);
}

#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
struct TestModel {
    name: String,
    field1: String,
    field2: i32,
}

impl HasName for TestModel {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

fn attach_entry_name<T: HasName>(entry_name: String, mut content: T) -> T {
    content.set_name(entry_name);
    content
}

fn bench_attach_entry_name(c: &mut Criterion) {
    c.bench_function("attach_entry_name", |b| {
        let content = TestModel {
            field1: "test".to_string(),
            field2: 42,
            ..Default::default()
        };
        b.iter(|| {
            let result = attach_entry_name("test_entry".to_string(), black_box(content.clone()));
            black_box(result)
        })
    });
}

// Batch attach_entry_name
fn bench_attach_entry_name_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("attach_entry_name_batch");

    for size in [10, 100, 1000] {
        group.bench_function(format!("batch_{}", size), |b| {
            let contents: Vec<TestModel> = (0..size)
                .map(|i| TestModel {
                    name: String::new(),
                    field1: format!("field_{}", i),
                    field2: i,
                })
                .collect();

            b.iter(|| {
                contents
                    .iter()
                    .enumerate()
                    .map(|(i, c)| attach_entry_name(format!("entry_{}", i), black_box(c.clone())))
                    .collect::<Vec<_>>()
            })
        });
    }

    group.finish();
}

// Simulate extracting all entry names from a list response
fn extract_all_entry_names(resp: &serde_json::Value) -> Vec<String> {
    resp.get("entry")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn bench_extract_all_entry_names(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_all_entry_names");

    for size in [10, 100, 1000] {
        let response = generate_entry_response(size);
        group.bench_function(format!("entries_{}", size), |b| {
            b.iter(|| black_box(extract_all_entry_names(black_box(&response))))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_entry_content_single,
    bench_extract_entry_content_10,
    bench_extract_entry_content_100,
    bench_extract_entry_content_1k,
    bench_attach_entry_name,
    bench_attach_entry_name_batch,
    bench_extract_all_entry_names
);
criterion_main!(benches);
