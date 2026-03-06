# Reviewer Verification Checklist

Use this checklist to validate repository readiness on a fresh machine.

This path is intended for external reviewers who want to verify engineering quality, operational trustworthiness, and real-world usability without having to reverse-engineer the maintainer workflow first.

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

## 3) Run Fast Local Gate

```bash
make ci-fast
```

Expected:

- exit code `0`
- clear stage-by-stage output
- no local source mutations from the gate itself

## 4) Run Full Local Gate

```bash
make ci
```

## 5) Optional Strict Live Validation

Requires a reachable Splunk instance and configured credentials.

```bash
CI_LIVE_TESTS_MODE=required make ci
```

## 6) Qualitative Confidence Checks (High Signal)

### A) Help-output secret safety

```bash
cargo test -p splunk-cli --test config_loading_tests test_help_hides_env_var_values
cargo test -p splunk-tui --test cli_tests test_help_hides_env_var_values
```

### B) Search UX normalization

```bash
cargo test -p splunk-cli --test search_tests
```

### C) TUI resize stability

```bash
cargo test -p splunk-tui --test resize_tests
```

### D) Style-aware visual regression + interaction checks

```bash
make tui-visual
```

### E) Theme accessibility contrast checks

```bash
make tui-accessibility
```

### F) Live smoke for core journeys (if Splunk available)

```bash
splunk-cli doctor
splunk-cli --output json health
splunk-cli --output json search "index=_internal | head 5" --wait --count 1
splunk-cli --output json apps list --count 5
splunk-cli --output json jobs --count 5
```

### G) Live JSON shape sanity checks (high signal)

```bash
splunk-cli --output json health > health.json
splunk-cli --output json doctor > doctor.json

python3 - <<'PY'
import json, pathlib

samples = {
  "health": ("health.json", ["server_info", "splunkd_health"]),
  "doctor": ("doctor.json", ["cli_version", "checks"]),
}

for label, (path, keys) in samples.items():
  data = json.loads(pathlib.Path(path).read_text())
  missing = [k for k in keys if k not in data]
  if missing:
    raise SystemExit(f"{label} missing keys: {missing}")

print("live JSON shape sanity checks passed")
PY
```

## 7) Additional Confidence Checks

```bash
# architecture invariants
cargo test -p architecture-tests

# fast UI regression suite
make tui-smoke
make tui-visual
make tui-accessibility

# chaos/resilience tests
make test-chaos

# secret tracking guard
make lint-secrets
```

## 8) Repository Hygiene Checks

```bash
# no unexpected tracked artifacts
git ls-files logs/ .ralph/ crates/tui/logs/

# clean working tree after checks
git status --porcelain
```

Both commands should produce no output in a healthy state.

## 9) Build + Installable Binaries (Optional)

```bash
make build
make install-bins

splunk-cli --version
splunk-tui --version
```

## 10) Documentation Drift and Links

```bash
make lint-docs
cargo test -p architecture-tests --test docs_link_validation_tests
```

Expected: pass with no broken references.
