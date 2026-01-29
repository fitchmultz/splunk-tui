//! Benchmark for measuring tokio::spawn overhead in side_effects.
//!
//! This benchmark quantifies the overhead of spawning tasks versus direct
//! async function calls, providing baseline data for optimization decisions.
//!
//! Run with: cargo bench -p splunk-tui --bench spawn_overhead

use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simulated shared state similar to SplunkClient wrapper.
struct SharedState {
    value: u64,
}

/// Simulated async operation that acquires a lock and does work.
async fn simulated_api_call(state: Arc<Mutex<SharedState>>) -> u64 {
    let mut guard = state.lock().await;
    // Simulate a small amount of work (like parsing JSON response)
    guard.value += 1;
    guard.value
}

/// Benchmark task spawn overhead with shared mutex.
/// This version waits for all tasks to complete to avoid race conditions.
async fn benchmark_spawn_with_mutex(iterations: usize) {
    let state = Arc::new(Mutex::new(SharedState { value: 0 }));
    let mut handles = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            simulated_api_call(state_clone).await
        }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }

    let final_state = state.lock().await;
    assert_eq!(final_state.value, iterations as u64);
}

/// Benchmark direct async calls with shared mutex (no spawn).
async fn benchmark_direct_with_mutex(iterations: usize) {
    let state = Arc::new(Mutex::new(SharedState { value: 0 }));

    for _ in 0..iterations {
        let state_clone = state.clone();
        // Direct call without spawn
        let _ = simulated_api_call(state_clone).await;
    }

    let final_state = state.lock().await;
    assert_eq!(final_state.value, iterations as u64);
}

/// Benchmark sequential spawns (tasks complete before next spawn).
async fn benchmark_sequential_spawns(iterations: usize) {
    let state = Arc::new(Mutex::new(SharedState { value: 0 }));

    for _ in 0..iterations {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move { simulated_api_call(state_clone).await });
        // Wait for each task to complete before spawning next
        let _ = handle.await.unwrap();
    }

    let final_state = state.lock().await;
    assert_eq!(final_state.value, iterations as u64);
}

/// Benchmark concurrent spawns (all tasks spawned, then awaited).
async fn benchmark_concurrent_spawns(iterations: usize) {
    let state = Arc::new(Mutex::new(SharedState { value: 0 }));
    let mut handles = Vec::with_capacity(iterations);

    // Spawn all tasks first
    for _ in 0..iterations {
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            simulated_api_call(state_clone).await
        }));
    }

    // Then await all
    for handle in handles {
        let _ = handle.await.unwrap();
    }

    let final_state = state.lock().await;
    assert_eq!(final_state.value, iterations as u64);
}

fn spawn_overhead_benchmarks(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("spawn_overhead");

    // Benchmark: Direct async calls (baseline)
    group.bench_function("direct_calls_100", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_direct_with_mutex(100))
    });

    // Benchmark: Sequential spawns (spawn + immediate await)
    group.bench_function("sequential_spawns_100", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_sequential_spawns(100))
    });

    // Benchmark: Concurrent spawns (all spawned, then awaited)
    // This simulates the current side_effects pattern
    group.bench_function("concurrent_spawns_100", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_concurrent_spawns(100))
    });

    // Benchmark: Fire-and-forget spawns (no await)
    // Measures pure spawn overhead
    group.bench_function("fire_and_forget_100", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_spawn_with_mutex(100))
    });

    // Scale tests - how does overhead grow with iteration count?
    group.bench_function("concurrent_spawns_10", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_concurrent_spawns(10))
    });

    group.bench_function("concurrent_spawns_50", |b| {
        b.to_async(&runtime)
            .iter(|| benchmark_concurrent_spawns(50))
    });

    group.finish();
}

criterion_group!(benches, spawn_overhead_benchmarks);
criterion_main!(benches);
