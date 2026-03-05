# Operational Readiness Checklist

## Configuration

- [ ] `.env` and `.env.test` are untracked
- [ ] Real credentials are configured (no placeholders)
- [ ] TLS verification policy is explicitly chosen

## Build and Test

- [ ] `make ci-fast` passes
- [ ] `make ci` passes
- [ ] `make lint-secrets` passes

## Runtime and Observability

- [ ] `splunk-cli doctor` succeeds in target environment
- [ ] Logging path and retention are validated
- [ ] Optional metrics endpoint tested if enabled

## Release Controls

- [ ] `docs/reviewer-verification.md` steps pass on fresh clone
- [ ] History cutover runbook executed before public flip
- [ ] Branch protections enabled after public launch

## Rollback Preparedness

- [ ] Pre-cutover bundle backup created and verified
- [ ] Rollback SHA recorded and tested in runbook procedure
