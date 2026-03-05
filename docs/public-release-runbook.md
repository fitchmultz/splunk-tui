# Public Release Commit Series and History Rewrite Runbook

This document provides a clean commit plan for the current hardening delta and a safe procedure to rewrite history before making the repository public.

## Scope and Assumptions

- Repository is still private.
- Current hardening delta is `17 modified + 12 untracked` files (`29` total).
- Local quality gate is `make ci`.
- Conventional commits are the public commit style.

## Part 1 — Ordered, Squash-Ready Commit Series

Create a branch for the series:

```bash
git switch main
git pull --ff-only
git switch -c prep/public-release-series
```

### Commit 1

```bash
git add crates/tui/tests/undo_tests.rs
git commit -m "test(tui): stabilize undo integration tests"
cargo test -p splunk-tui --test undo_tests
```

### Commit 2

```bash
git add Makefile scripts/validate-live-test-env.sh .cargo/config.toml .env.test.example
git commit -m "ci(make): harden deterministic local gates"
make ci-fast
```

### Commit 3

```bash
git add CHANGELOG.md CODE_OF_CONDUCT.md
git commit -m "docs: add governance docs for public readiness"
```

### Commit 4

```bash
git add docs/index.md docs/architecture.md docs/ci.md
git commit -m "docs: add architecture and CI documentation hub"
```

### Commit 5

```bash
git add docs/reviewer-verification.md docs/release-readiness.md
git commit -m "docs: add reviewer verification and release readiness report"
```

### Commit 6

```bash
git add README.md CONTRIBUTING.md docs/contributing.md docs/testing.md docs/usage.md \
  docs/user-guide.md docs/containers.md AGENTS.md
git commit -m "docs: align contributor and user docs with release contract"
make ci-fast
```

### Commit 7

```bash
git add crates/architecture-tests/tests/docs_link_validation_tests.rs \
  crates/architecture-tests/tests/exit_code_docs_tests.rs \
  crates/architecture-tests/tests/repo_artifacts_tests.rs
git commit -m "test(architecture-tests): enforce docs and repo hygiene invariants"
cargo test -p architecture-tests
make ci
```

### Optional squash profile for public history

If you want fewer, higher-signal public commits, squash as:

1. `test(tui)` + both `ci(...)` commits → **CI/Test foundation**
2. Governance + all docs commits → **Public documentation package**
3. Architecture-tests commit → **Drift-prevention enforcement**

## Part 2 — Safe History Rewrite Runbook (Hard Cutover)

This procedure rewrites history so public viewers see only curated history.

## Safety Rules

- Run rewrite in a separate clone.
- Keep offline/private backups before any force push.
- Do not push backup refs to the future public remote.

### Step 0: Pre-flight checks

```bash
make lint-secrets
make ci-fast
make ci
git status --porcelain
```

Expected: no failures and clean working tree.

### Step 1: Anchor and back up private history

From the repo containing `prep/public-release-series`:

```bash
BASE_COMMIT="$(git merge-base main prep/public-release-series)"
SERIES_HEAD="$(git rev-parse prep/public-release-series)"
OLD_MAIN="$(git rev-parse origin/main)"

git tag -f pre-public-base "$BASE_COMMIT"
git tag -f pre-public-head "$SERIES_HEAD"

mkdir -p ../repo-backups
BUNDLE="../repo-backups/splunk-tui-pre-public-$(date +%Y%m%d).bundle"
git bundle create "$BUNDLE" --all
git bundle verify "$BUNDLE"

echo "$OLD_MAIN" > ../repo-backups/splunk-tui-old-main.sha
```

### Step 1.5: Audit baseline tree content before rewrite

Before creating the new public root commit from `BASE`, verify the baseline tree does not contain sensitive files or internal-only artifacts.

```bash
# Verify forbidden secret files are absent in baseline tree
git ls-tree -r --name-only "$BASE_COMMIT" | grep -qE '(^|/)\.env$|(^|/)\.env\.test$' && {
  echo "✗ Baseline commit contains forbidden env files (.env / .env.test)";
  exit 1;
} || true

# Optional: inspect baseline tree for internal-only docs/notes before publishing
git ls-tree -r --name-only "$BASE_COMMIT" | grep -E '(^|/)(internal|private|draft)' || true
```

If anything sensitive is found, stop and choose a different baseline/cutover strategy before proceeding.

### Step 2: Build rewritten history in isolated clone

```bash
git clone --no-local . ../splunk-tui-public-rewrite
cd ../splunk-tui-public-rewrite
git fetch --tags origin

BASE="$(git rev-parse pre-public-base)"
HEAD="$(git rev-parse pre-public-head)"
PATCH="/tmp/splunk-tui-public-series.patch"

git format-patch --stdout "$BASE..$HEAD" > "$PATCH"

git switch --orphan public-main
git rm -rf . >/dev/null 2>&1 || true
git checkout "$BASE" -- .
git add -A
git commit -m "chore(repo): initialize public baseline"
git am "$PATCH"
```

Result: public branch has a fresh root plus the curated series; old private history is not reachable.

### Step 3: Validate rewritten branch

```bash
make lint-secrets
make ci-fast
make ci
git log --oneline --decorate -n 20
```

### Step 4: Force-push with lease

```bash
EXPECTED_OLD_MAIN="$(git ls-remote origin refs/heads/main | awk '{print $1}')"
git push origin public-main:main --force-with-lease=refs/heads/main:"$EXPECTED_OLD_MAIN"
```

### Step 5: Post-push verification

```bash
cd ..
git clone <public-repo-url> splunk-tui-public-verify
cd splunk-tui-public-verify
make ci-fast
```

Then flip repository visibility to public and enable branch protections.

## Rollback Procedure

If validation fails after force-push, restore old `main` from the saved SHA:

```bash
OLD_MAIN="$(cat ../repo-backups/splunk-tui-old-main.sha)"
git push origin "$OLD_MAIN":refs/heads/main --force-with-lease
```

If old objects are unavailable, recover from the bundle backup:

```bash
git clone ../repo-backups/splunk-tui-pre-public-YYYYMMDD.bundle splunk-tui-restore
```

## Final Public-Readiness Checklist

- [ ] Commit series applied and validated (`make ci` passes)
- [ ] Private history bundle created and verified
- [ ] Rewritten branch validated in isolated clone
- [ ] Force-push performed with lease
- [ ] Fresh clone verification passed
- [ ] Repository visibility switched to public
- [ ] Branch protection rules enabled
