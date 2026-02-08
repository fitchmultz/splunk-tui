//! Benchmarks for SPL syntax highlighting.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use splunk_config::Theme;
use splunk_tui::ui::syntax::highlight_spl;

fn simple_query() -> &'static str {
    "search index=main | stats count by sourcetype"
}

fn complex_query() -> &'static str {
    r#"search index=_internal source=*splunkd.log earliest=-24h 
    | eval hour=strftime(_time, "%H") 
    | stats count by hour, component 
    | sort -count 
    | head 20 
    | eval percentage=round(count/total*100, 2)"#
}

fn very_complex_query() -> &'static str {
    r#"search index=main (error OR fail* OR exception) 
    | eval severity=case(
        match(_raw, "(?i)critical"), "critical",
        match(_raw, "(?i)error"), "error",
        match(_raw, "(?i)warning"), "warning",
        1=1, "info"
    )
    | stats count, values(host) as hosts by severity, source
    | sort -count
    | head 100
    | eval host_count=mvcount(hosts)
    | where count > 10
    | table severity, source, count, host_count, hosts
    | addcoltotals count
    | append [search index=main | stats count as total | eval severity="TOTAL"]
    | sort -count"#
}

fn bench_highlight_simple(c: &mut Criterion) {
    let theme = Theme::default();
    c.bench_function("highlight_simple", |b| {
        b.iter(|| black_box(highlight_spl(black_box(simple_query()), &theme)))
    });
}

fn bench_highlight_complex(c: &mut Criterion) {
    let theme = Theme::default();
    c.bench_function("highlight_complex", |b| {
        b.iter(|| black_box(highlight_spl(black_box(complex_query()), &theme)))
    });
}

fn bench_highlight_very_complex(c: &mut Criterion) {
    let theme = Theme::default();
    c.bench_function("highlight_very_complex", |b| {
        b.iter(|| black_box(highlight_spl(black_box(very_complex_query()), &theme)))
    });
}

// Batch highlighting
fn bench_highlight_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("highlight_batch");
    let theme = Theme::default();
    let queries: Vec<&str> = vec![
        simple_query(),
        complex_query(),
        very_complex_query(),
        "search index=* | head 10",
        "stats count",
        "timechart span=1h count",
    ];

    group.bench_function("batch_6_queries", |b| {
        b.iter(|| {
            for query in &queries {
                let _ = black_box(highlight_spl(black_box(query), &theme));
            }
        })
    });

    group.finish();
}

// Test highlighting of multiline queries
fn bench_highlight_multiline(c: &mut Criterion) {
    let theme = Theme::default();
    let query = r#"search index=main
    | eval x=1
    | eval y=2
    | eval z=3
    | stats count by x, y, z"#;

    c.bench_function("highlight_multiline", |b| {
        b.iter(|| black_box(highlight_spl(black_box(query), &theme)))
    });
}

// Test highlighting with strings and macros
fn bench_highlight_with_strings(c: &mut Criterion) {
    let theme = Theme::default();
    let query = r#"search index=main message="This is a test message with \"quotes\"" | `my_macro` | stats count"#;

    c.bench_function("highlight_with_strings", |b| {
        b.iter(|| black_box(highlight_spl(black_box(query), &theme)))
    });
}

criterion_group!(
    benches,
    bench_highlight_simple,
    bench_highlight_complex,
    bench_highlight_very_complex,
    bench_highlight_batch,
    bench_highlight_multiline,
    bench_highlight_with_strings
);
criterion_main!(benches);
