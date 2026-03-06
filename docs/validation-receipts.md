# Validation Receipts

## Context

- Date: **2026-03-05**
- Branch at time of capture: `public-ready-final`

## Local Quality Gate Receipts

### Secret guard

```bash
make lint-secrets
```

Result:

- Pass
- Message: `Secret-commit guard OK: no forbidden paths are tracked.`

### Fast local gate

```bash
make ci-fast
```

Result:

- Pass
- Warm-cache wall time observed previously: about 25 seconds

### Full gate (default)

```bash
make ci
```

Result:

- Pass
- Includes full workspace tests, docs drift checks, and CI-profile build.

### Full gate (strict live)

```bash
CI_LIVE_TESTS_MODE=required make ci
```

Result:

- Pass
- Client live tests: **21 passed**
- CLI live tests: **15 passed**

## Qualitative Dogfood Receipt

Reference:

- `docs/dogfood-2026-03-05.md`

Key outcomes:

- Help-output env value leakage fixed and covered by regression tests
- Plaintext config flow decryption/profile-load friction fixed and covered by regression tests
- Bare `index=...` search UX fixed via normalization and tests
- TUI remained stable under resize stress with no panic or error observed

## Git Hygiene

Verification command:

```bash
git status --porcelain
```

Expected result before release tag or push:

- No unexpected changes
