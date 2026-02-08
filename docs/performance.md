# Performance Baselines and Benchmarks

This document describes the performance benchmarks for splunk-tui and established baselines.

## Running Benchmarks

```bash
# Run all benchmarks
make bench

# Run benchmarks for specific crates
make bench-client
make bench-cli
make bench-tui

# Run specific benchmark file
cargo bench -p splunk-client --bench serde_helpers_benchmarks
```

## Benchmark Suites

### Serde Helpers (`crates/client/benches/serde_helpers_benchmarks.rs`)

Benchmarks for Splunk's inconsistent JSON typing helpers.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `u64_from_number` | Parse numeric u64 field | < 1µs |
| `u64_from_string` | Parse string u64 field | < 1µs |
| `usize_from_number` | Parse numeric usize field | < 1µs |
| `usize_from_string` | Parse string usize field | < 1µs |
| `batch_100_usize` | Batch deserialize 100 items | < 100µs |
| `batch_1000_usize` | Batch deserialize 1000 items | < 1ms |

### Model Parsing (`crates/client/benches/model_parsing_benchmarks.rs`)

Benchmarks for JSON deserialization of API models.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `search_job_results_1k` | Parse 1000 search results | < 10ms |
| `search_job_results_10k` | Parse 10,000 search results | < 100ms |
| `search_job_results_50k` | Parse 50,000 search results | < 500ms |
| `index_list_100` | Parse 100 indexes | < 5ms |
| `index_list_1k` | Parse 1000 indexes | < 50ms |
| `cluster_info` | Parse cluster info response | < 100µs |
| `parse_10mb_json` | Parse ~10MB JSON payload | < 100ms |

### Entry Extraction (`crates/client/benches/entry_extraction_benchmarks.rs`)

Benchmarks for Splunk response extraction helpers.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `extract_entry_single` | Extract first entry | < 1µs |
| `extract_entry_10` | Extract from 10 entries | < 1µs |
| `extract_entry_100` | Extract from 100 entries | < 1µs |
| `extract_entry_1k` | Extract from 1000 entries | < 1µs |
| `attach_entry_name` | Attach name to model | < 1µs |
| `attach_entry_name_batch/batch_10` | Batch attach 10 names | < 10µs |
| `attach_entry_name_batch/batch_100` | Batch attach 100 names | < 100µs |
| `attach_entry_name_batch/batch_1000` | Batch attach 1000 names | < 1ms |

### Formatters (`crates/cli/benches/formatter_benchmarks.rs`)

Benchmarks for CLI output formatting.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `csv_logs_100` | Format 100 log entries as CSV | < 1ms |
| `csv_logs_1k` | Format 1000 log entries as CSV | < 5ms |
| `csv_search_results_1k` | Format 1000 search results as CSV | < 5ms |
| `csv_wide_1000x50` | Format 1000x50 dataset as CSV | < 10ms |
| `table_logs_100` | Format 100 log entries as table | < 1ms |
| `table_logs_1k` | Format 1000 log entries as table | < 5ms |
| `json_logs_100` | Format 100 log entries as JSON | < 1ms |
| `json_logs_1k` | Format 1000 log entries as JSON | < 5ms |
| `json_logs_10k` | Format 10,000 log entries as JSON | < 50ms |

### Syntax Highlighting (`crates/tui/benches/syntax_highlighting_benchmarks.rs`)

Benchmarks for SPL syntax highlighting.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `highlight_simple` | Simple SPL query | < 50µs |
| `highlight_complex` | Complex SPL query | < 200µs |
| `highlight_very_complex` | Very complex SPL | < 500µs |
| `highlight_batch/batch_6_queries` | Batch 6 queries | < 1ms |
| `highlight_multiline` | Multiline query | < 100µs |
| `highlight_with_strings` | Query with strings | < 100µs |

### Export (`crates/tui/benches/export_benchmarks.rs`)

Benchmarks for export operations.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `export_json/json_100` | Export 100 results to JSON | < 5ms |
| `export_json/json_1000` | Export 1000 results to JSON | < 20ms |
| `export_json/json_10000` | Export 10000 results to JSON | < 100ms |
| `export_csv/csv_100` | Export 100 results to CSV | < 5ms |
| `export_csv/csv_1000` | Export 1000 results to CSV | < 20ms |
| `export_csv/csv_10000` | Export 10000 results to CSV | < 100ms |
| `export_csv_wide/csv_1000x50` | Export 1000x50 dataset | < 50ms |

## CI Integration

Benchmarks compile in CI to ensure they don't break. To detect performance regressions locally:

```bash
# Run benchmarks and save baseline
cargo bench -p splunk-client -- --save-baseline main

# Make changes, then compare
cargo bench -p splunk-client -- --baseline main
```

A change that causes:
- > 10% regression in any benchmark requires investigation
- > 50% regression blocks the PR

## Profiling

For detailed profiling of benchmarks:

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph for a benchmark
cargo flamegraph --bench serde_helpers_benchmarks
```

## Implementation Notes

### Visibility Requirements

The `serde_helpers` module is exposed via the `test-utils` feature for benchmark access. Benchmarks in `crates/client/benches/` automatically have access to these internals.

### Test Data Generation

Benchmarks use generated test data rather than fixtures for:
- Consistent sizing (1k, 10k, 100k rows)
- Controlled data shapes (wide datasets)
- Deterministic performance characteristics

### Async Benchmarks

TUI benchmarks that require async use Criterion's `to_async` method with a Tokio runtime:

```rust
let runtime = tokio::runtime::Runtime::new().unwrap();
group.bench_function("export", |b| {
    b.to_async(&runtime).iter(|| async {
        // async code here
    })
});
```
