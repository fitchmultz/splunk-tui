# 45–60 Minute Workshop Outline

## Audience

Engineers validating repo readiness for public release.

## Agenda

1. **Context and architecture (10 min)**
   - Walk through `docs/architecture.md`
   - Explain CLI/TUI parity via shared `crates/client`

2. **CI contract and determinism (15 min)**
   - Run `make ci-fast`
   - Inspect `Makefile` and `docs/ci.md`

3. **Security and release discipline (10 min)**
   - Run `make lint-secrets`
   - Review `SECURITY.md` and history-cutover runbook

4. **Hands-on validation lab (15 min)**
   - Participants run `make ci`
   - Compare outcomes and failure triage patterns

5. **Debrief (5–10 min)**
   - Remaining risk: history cutover execution
   - Public release checklist and ownership

## Success Criteria

- Participants can run and interpret both CI gates
- Participants can identify where security guardrails live
- Participants can follow the release cutover runbook safely

## Failure Modes to Discuss

- Failing secret guard
- Docs drift failures
- Non-deterministic live-test expectations
