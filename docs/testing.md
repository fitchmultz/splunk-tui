# Testing Methodology

This document describes the testing strategies used in the Splunk TUI project, with a focus on chaos engineering and fault injection techniques.

## Test Categories

### Unit Tests

Unit tests are located inline with the code they test, using `#[cfg(test)]` modules. They test individual functions and small components in isolation.

### Integration Tests

Integration tests are located in `crates/client/tests/*.rs`. Each file focuses on a specific testing concern such as authentication, job management, or search operations.

### Chaos Engineering Tests

Chaos tests verify system resilience under adverse conditions. They use Wiremock to simulate failures and verify the client handles them gracefully.

## Chaos Test Scenarios

### Network Chaos (`chaos_network_tests.rs`)

Tests network-level failures:

| Scenario | Description | Expected Behavior |
|----------|-------------|-------------------|
| Truncated JSON | Connection drop mid-response | Return error immediately (not retryable) |
| Malformed JSON | Invalid JSON body | Return error immediately (not retryable) |
| Empty Response | HTTP 200 with no body | Return error immediately (not retryable) |
| Connection Error | 503 Service Unavailable | Retry after exponential backoff |
| Partial Schema | Valid JSON but missing fields | Return `InvalidResponse` error (not retryable) |
| Unexpected Content | text/plain instead of JSON | Return error immediately (not retryable) |
| Large Responses | 1000+ entry results | Handle without memory issues |

### Timing Chaos (`chaos_timing_tests.rs`)

Tests time-related edge cases:

| Scenario | Description | Expected Behavior |
|----------|-------------|-------------------|
| Session Expiry Re-authentication | Session expires during operation | Re-authenticate and retry request |
| Session During Pagination | Expiry mid-pagination | Re-auth and continue pagination |
| Repeated Session Failures | Server keeps rejecting session | Re-authenticate once, then fail |
| API Token No Re-auth | API token receives 401 | Fail immediately (no re-auth for API tokens) |
| 403 Triggers Re-auth | Forbidden response for session | Re-authenticate and retry |
| Session Expiry During Job Creation | Expiry while creating search job | Re-authenticate and retry |

### Flapping Chaos (`chaos_flapping_tests.rs`)

Tests rapid state changes:

| Scenario | Description | Expected Behavior |
|----------|-------------|-------------------|
| 200/503 Flapping | Rapid healthy/unhealthy | Eventual convergence |
| Rate Limit Flapping | Rapid 429/200 changes | Respect Retry-After, succeed |
| Cascading Failures | Gradual error recovery | Retry until success |
| Random Chaos | Mixed status codes | General resilience |
| Load Balancer Flapping | Multiple backend states | Retry until healthy backend |
| Flapping Exhaustion | Continuous errors | Fail after max retries |
| Varying Error Messages | Different error texts | Retry regardless of message |

## Running Chaos Tests

### Run All Chaos Tests

```bash
make test-chaos
```

Or using cargo directly:

```bash
cargo test -p splunk-client --features test-utils chaos_
```

### Run Specific Chaos Test File

```bash
# Network chaos
cargo test -p splunk-client --test chaos_network_tests --features test-utils

# Timing chaos
cargo test -p splunk-client --test chaos_timing_tests --features test-utils

# Flapping chaos
cargo test -p splunk-client --test chaos_flapping_tests --features test-utils
```

### Run with Output

```bash
cargo test -p splunk-client --test chaos_network_tests --features test-utils -- --nocapture
```

### Run a Specific Test

```bash
cargo test -p splunk-client --test chaos_network_tests --features test-utils test_truncated_json_response -- --nocapture
```

## Test Patterns

### Using Paused Time

Chaos tests use Tokio's paused time for deterministic testing:

```rust
#[tokio::test(start_paused = true)]
async fn test_with_paused_time() {
    // Test code here
    advance_and_yield(Duration::from_secs(1)).await;
    // More test code
}
```

The `advance_and_yield()` helper is available from `common/mod.rs` and combines time advancement with a task yield to ensure pending tasks can observe the time change.

### Request Counting

Track request counts to verify retry behavior:

```rust
let request_count = Arc::new(AtomicUsize::new(0));
let count_clone = request_count.clone();

Mock::given(method("GET"))
    .and(path("/endpoint"))
    .respond_with(move |_req| {
        let count = count_clone.fetch_add(1, Ordering::SeqCst);
        // Respond based on count
        if count == 0 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200)
        }
    })
    .mount(&mock_server)
    .await;

// After test
assert_eq!(request_count.load(Ordering::SeqCst), 2, "Should retry once");
```

### Response Templates

Wiremock's `ResponseTemplate` supports:

- **Status codes**: `ResponseTemplate::new(503)`
- **JSON bodies**: `.set_body_json(json!(...))`
- **String bodies**: `.set_body_string("...")`
- **Headers**: `.insert_header("name", "value")`
- **Delays**: `.set_delay(Duration::from_secs(1))`
- **Limited responses**: `.up_to_n_times(1)`

### Sequential Responses

Use `up_to_n_times()` to create sequential responses:

```rust
// First two requests fail
Mock::given(method("GET"))
    .and(path("/endpoint"))
    .respond_with(ResponseTemplate::new(503))
    .up_to_n_times(2)
    .mount(&mock_server)
    .await;

// Subsequent requests succeed
Mock::given(method("GET"))
    .and(path("/endpoint"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&mock_server)
    .await;
```

## Retry Behavior Reference

The client implements exponential backoff with the following characteristics:

| Attempt | Delay |
|---------|-------|
| 1 (initial) | 0s |
| 2 | 1s |
| 3 | 2s |
| 4 | 4s |
| 5 | 8s |

**Retryable status codes**: 429, 502, 503, 504

**Transport errors** (also retryable):
- Timeouts
- Connection refused
- Connection reset
- Broken pipe
- DNS failures

The client respects the `Retry-After` header for 429 responses, using the maximum of the calculated backoff and the header value. Both delay-seconds format (e.g., "120") and HTTP-date format (e.g., "Wed, 21 Oct 2015 07:28:00 GMT") are supported per RFC 7231.

## Session Management

Session-based authentication includes:

- **Proactive refresh**: Tokens are refreshed before expiry based on `session_ttl_seconds` and `session_expiry_buffer_seconds`
- **Reactive refresh**: On 401/403 responses, the client re-authenticates and retries the request
- **Session TTL**: Default 3600 seconds (1 hour)
- **Expiry buffer**: Default 60 seconds (refresh if token expires within this window)

## Adding New Chaos Tests

When adding new chaos tests:

1. **Choose the appropriate test file** based on the failure type:
   - Network issues → `chaos_network_tests.rs`
   - Timing issues → `chaos_timing_tests.rs`
   - State flapping → `chaos_flapping_tests.rs`

2. **Use `#[tokio::test(start_paused = true)]`** for time-dependent tests

3. **Document the scenario** in this file

4. **Verify the test fails** without the resilience feature being tested

5. **Run the PR gate** before committing:
   ```bash
   make ci-fast
   ```

## Test Utilities

The `common/mod.rs` module provides shared utilities:

- `load_fixture(path)` - Load JSON fixtures from the fixtures directory
- `advance_and_yield(duration)` - Advance paused time and yield
- `assert_pending(handle, context)` - Assert a task has not completed

## Fixtures

Test fixtures are stored in `crates/client/fixtures/`:

- `auth/login_success.json` - Successful authentication response
- `auth/login_invalid_creds.json` - Failed authentication response
- `server/get_server_info.json` - Server info response
- `search/get_results.json` - Search results response
- And more...

## CI Integration

Chaos tests are part of the full gate (`make ci`) and can be run independently. Live tests remain skipped by default in `make ci` (`CI_LIVE_TESTS_MODE=skip`) for deterministic offline execution.

```bash
# Run only chaos tests
make test-chaos

# Run as part of full test suite
make test
```

## Debugging Failed Tests

If a chaos test fails:

1. **Check the failure type**: Is it a timeout, parse error, or unexpected success?

2. **Review the retry count**: Did the client retry the expected number of times?

3. **Examine timing**: For paused time tests, did you advance time enough?

4. **Enable logging**: Run with `-- --nocapture` and set `RUST_LOG=debug`:
   ```bash
   RUST_LOG=debug cargo test -p splunk-client --test chaos_network_tests --features test-utils -- --nocapture
   ```

5. **Verify mock setup**: Are the mock responses configured correctly with `up_to_n_times()`?

## TUI UX Regression Suite

The TUI includes a comprehensive snapshot-based UX regression suite to prevent silent behavior drift in critical user-facing surfaces.

### Test Files

| File | Tests | Purpose |
|------|-------|---------|
| `snapshot_tutorial_tests.rs` | 8 | Tutorial wizard popup (all 7 steps + small terminal) |
| `snapshot_error_details_tests.rs` | 5 | ErrorDetails popup (basic, context, messages, JSON, scrollable) |
| `snapshot_popups_tests.rs` | 20 | Auth recovery, connection diagnostics, help, confirm, index details |
| `snapshot_footer_tests.rs` | 18 | Footer hints per screen/mode, popup context hints |
| `first_run_tests.rs` | 10 | First-run detection, tutorial wiring, persistence |
| `tutorial_app_tests.rs` | 17 | Tutorial action handling, state persistence |
| `app_error_tests.rs` | 31 | Error classification, auth recovery flows |
| `app_navigation_tests.rs` | 23 | Screen cycling, list navigation |

### Running TUI UX Tests

```bash
# Run all character-only snapshot tests
cargo test -p splunk-tui --test snapshot_

# Run style-aware + interaction visual tests
make tui-visual

# Run accessibility contrast checks
make tui-accessibility

# Run specific UX test files
cargo test -p splunk-tui --test snapshot_tutorial_tests
cargo test -p splunk-tui --test snapshot_error_details_tests
cargo test -p splunk-tui --test first_run_tests
```

### Snapshot Best Practices

For style-aware snapshots (`snapshot_styled_tests.rs`), use `harness.render_styled()` so each row includes compact style runs (`fg/bg/modifiers`) and can catch semantic color regressions.


1. **Determinism**: Avoid timestamps, random data, or HashMap ordering in snapshots
2. **Terminal size**: Use appropriate sizes (80x24 for standard, 120x50 for complex popups)
3. **Review changes**: When snapshots change, use `cargo insta review` to accept/reject

### Adding New UX Snapshots

1. Create a `TuiHarness` with appropriate terminal dimensions
2. Set up the app state (popup, current_error, etc.)
3. For text/layout checks, call `harness.render()` and snapshot the output
4. For style/semantic checks, call `harness.render_styled()` and snapshot the output
5. Add targeted assertions (`assert_text_has_fg`, `assert_text_has_modifier`) for high-signal semantics
6. Run `cargo test` to generate initial snapshots
7. Review snapshots in `crates/tui/tests/snapshots/`

## Quick UX Validation

For rapid iteration on TUI visual components, use the smoke test target:

### TUI Smoke Tests

```bash
# Run only UX snapshot tests (~0.5 seconds)
make tui-smoke
```

This runs the 8 character-snapshot test files (84 tests total) covering:
- Tutorial wizard (8 tests)
- Error details (5 tests)
- Application popups (20 tests)
- Footer hints (21 tests)
- Screen layouts (12 tests)
- Search interfaces (10 tests)
- Job displays (5 tests)
- Miscellaneous UI (3 tests)

For stronger visual guarantees, run:

```bash
make tui-visual
make tui-accessibility
```

### When to Use Smoke vs Full CI

| Target | Speed | Coverage | When to Use |
|--------|-------|----------|-------------|
| `make tui-smoke` | ~0.5-1 min | UX snapshots only | Iterating on popups, layouts, visual styling |
| `make test` | ~5-30 min | Full workspace tests | Deep local verification |
| `make ci-fast` | ~8-20 min | Fast non-mutating local gate (smoke-focused) | During normal development |
| `make ci` | ~20-60 min | Full non-mutating local gate | Before release |

### Manual TUI Testing

For visual verification during development:

```bash
# Quick launch (requires SPLUNK_* env vars)
make run-tui

# Or directly
cargo run --package splunk-tui
```

## Visual Automation Roadmap (researched March 5, 2026)

Current approach (ratatui `TestBackend` + character/styled snapshots) should remain the primary PR gate because it is deterministic and fast.

For stronger visual automation beyond the current gate:

1. **Scripted terminal recordings for behavior demos + regression artifacts**
   - Use [VHS](https://github.com/charmbracelet/vhs) tapes to run repeatable CLI/TUI sessions.
   - VHS supports integration-test style output (`Output ...`) and works well as a local, manual capture tool.
2. **Stabilize snapshots with targeted redactions**
   - Use [Insta redactions](https://insta.rs/docs/redactions/) for dynamic fields (timestamps, IDs) when adding richer snapshots.
3. **Avoid adopting unmaintained renderers as required gates**
   - [termtosvg](https://github.com/nbedos/termtosvg) is useful for SVG captures but is currently read-only; keep it optional/manual if used.
4. **Optional capture tooling for manual audits**
   - [asciinema CLI](https://docs.asciinema.org/manual/cli/quick-start/) can record reproducible sessions for reviewer playback and troubleshooting artifacts.

## References

- [Wiremock Documentation](https://docs.rs/wiremock/)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Chaos Engineering Principles](https://principlesofchaos.org/)
- [Insta Snapshot Testing](https://insta.rs/)
- [Ratatui Snapshot Testing Recipe](https://ratatui.rs/recipes/testing/snapshots/)
- [VHS](https://github.com/charmbracelet/vhs)
