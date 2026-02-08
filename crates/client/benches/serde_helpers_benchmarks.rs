//! Benchmarks for serde helper functions.
//!
//! These helpers handle Splunk's inconsistent JSON typing (numbers as strings).

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serde::Deserialize;

// Wrapper structs for benchmarking the serde helpers
// Fields are used by serde but not directly accessed, hence allow(dead_code)
#[derive(Deserialize)]
#[allow(dead_code)]
struct U64Wrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::u64_from_string_or_number")]
    value: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct UsizeWrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::usize_from_string_or_number")]
    value: usize,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OptUsizeWrapper {
    #[serde(
        default,
        deserialize_with = "splunk_client::serde_helpers::opt_usize_from_string_or_number"
    )]
    value: Option<usize>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct StringOrNumberWrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::string_from_number_or_string")]
    value: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OptI32Wrapper {
    #[serde(
        default,
        deserialize_with = "splunk_client::serde_helpers::opt_i32_from_string_or_number"
    )]
    value: Option<i32>,
}

fn bench_u64_from_number(c: &mut Criterion) {
    c.bench_function("u64_from_number", |b| {
        let json = r#"{"value": 123456789}"#;
        b.iter(|| {
            let result: U64Wrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_u64_from_string(c: &mut Criterion) {
    c.bench_function("u64_from_string", |b| {
        let json = r#"{"value": "123456789"}"#;
        b.iter(|| {
            let result: U64Wrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_usize_from_number(c: &mut Criterion) {
    c.bench_function("usize_from_number", |b| {
        let json = r#"{"value": 123456789}"#;
        b.iter(|| {
            let result: UsizeWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_usize_from_string(c: &mut Criterion) {
    c.bench_function("usize_from_string", |b| {
        let json = r#"{"value": "123456789"}"#;
        b.iter(|| {
            let result: UsizeWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_opt_usize_some(c: &mut Criterion) {
    c.bench_function("opt_usize_some", |b| {
        let json = r#"{"value": 123456789}"#;
        b.iter(|| {
            let result: OptUsizeWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_opt_usize_none(c: &mut Criterion) {
    c.bench_function("opt_usize_none", |b| {
        let json = r#"{}"#;
        b.iter(|| {
            let result: OptUsizeWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_opt_i32_float(c: &mut Criterion) {
    c.bench_function("opt_i32_from_float", |b| {
        let json = r#"{"value": 12345.0}"#;
        b.iter(|| {
            let result: OptI32Wrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_string_from_number(c: &mut Criterion) {
    c.bench_function("string_from_number", |b| {
        let json = r#"{"value": 12345}"#;
        b.iter(|| {
            let result: StringOrNumberWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

fn bench_string_from_string(c: &mut Criterion) {
    c.bench_function("string_from_string", |b| {
        let json = r#"{"value": "hello"}"#;
        b.iter(|| {
            let result: StringOrNumberWrapper = serde_json::from_str(black_box(json)).unwrap();
            black_box(result)
        })
    });
}

// Batch benchmark for high-volume scenarios
fn bench_batch_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_serde_helpers");

    // Create a JSON array with 100 objects using string-or-number fields
    let json_100 = format!(
        "[{}]",
        (0..100)
            .map(|i| format!(r#"{{"value": "{}"}}"#, i))
            .collect::<Vec<_>>()
            .join(",")
    );

    // Create a JSON array with 1000 objects
    let json_1000 = format!(
        "[{}]",
        (0..1000)
            .map(|i| format!(r#"{{"value": "{}"}}"#, i))
            .collect::<Vec<_>>()
            .join(",")
    );

    group.bench_function("deserialize_100_usize", |b| {
        b.iter(|| {
            let results: Vec<UsizeWrapper> = serde_json::from_str(black_box(&json_100)).unwrap();
            black_box(results)
        })
    });

    group.bench_function("deserialize_1000_usize", |b| {
        b.iter(|| {
            let results: Vec<UsizeWrapper> = serde_json::from_str(black_box(&json_1000)).unwrap();
            black_box(results)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_u64_from_number,
    bench_u64_from_string,
    bench_usize_from_number,
    bench_usize_from_string,
    bench_opt_usize_some,
    bench_opt_usize_none,
    bench_opt_i32_float,
    bench_string_from_number,
    bench_string_from_string,
    bench_batch_deserialization
);
criterion_main!(benches);
