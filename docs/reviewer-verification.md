# Reviewer Verification Checklist

Use this checklist to validate repository readiness on a fresh machine.

## 1) Environment Setup

```bash
# clone + enter repo
git clone <repo-url>
cd splunk-tui

# toolchain sanity
rustc --version
cargo --version
make --version
```

## 2) Install Dependencies

```bash
make install
```

## 3) Run PR-Equivalent Gate

```bash
make ci-fast
```

Expected:

- exit code `0`
- clear stage-by-stage output
- no local source mutations from the gate itself

## 4) Run Full Mainline/Nightly Gate

```bash
make ci
```

## 5) Optional Strict Live Validation

Requires a reachable Splunk instance and configured credentials.

```bash
CI_LIVE_TESTS_MODE=required make ci
```

## 6) Additional Confidence Checks

```bash
# architecture invariants
cargo test -p architecture-tests

# fast UI regression suite
make tui-smoke

# chaos/resilience tests
make test-chaos

# secret tracking guard
make lint-secrets
```

## 7) Repository Hygiene Checks

```bash
# no unexpected tracked artifacts
git ls-files logs/ .ralph/ crates/tui/logs/

# clean working tree after checks
git status --porcelain
```

Both commands should produce no output in a healthy state.

## 8) Build + Installable Binaries (Optional)

```bash
make build
make install-bins

splunk-cli --version
splunk-tui --version
```

## 9) Documentation Drift and Links

```bash
make lint-docs
cargo test -p architecture-tests --test docs_link_validation_tests
```

Expected: pass with no broken references.
