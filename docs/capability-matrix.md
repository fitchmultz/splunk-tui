# Capability Matrix

This file is generated from `docs/capability-matrix.tsv`. Update the TSV source first, then keep this document in sync.

| Capability | Client | CLI | TUI | Rationale |
| --- | --- | --- | --- | --- |
| `search_jobs` | `read_write` | `yes` | `yes` | Shared search workflows power both frontends. |
| `saved_searches` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose management flows. |
| `indexes` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose index operations. |
| `users` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose user management. |
| `roles` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose role management. |
| `apps` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose app management. |
| `kvstore` | `read_write` | `yes` | `yes` | Shared client surface; both frontends expose KVStore status and collection flows. |
| `cluster_admin` | `read_write` | `yes` | `yes` | Shared cluster management APIs are exposed in both frontends. |
| `doctor_diagnostics` | `workflow` | `yes` | `yes` | Shared diagnostics workflow backs CLI doctor and TUI connection diagnostics. |
| `multi_profile_overview` | `workflow` | `yes` | `yes` | Shared multi-profile workflow backs CLI list-all and TUI multi-instance. |
| `structured_export` | `workflow` | `yes` | `yes` | Shared export workflow owns JSON/CSV/NDJSON/YAML/Markdown serialization. |
| `bootstrap_tutorial` | `ui_only` | `no` | `yes` | Interactive onboarding remains intentionally TUI-only. |
| `command_palette` | `ui_only` | `no` | `yes` | Interactive navigation remains intentionally TUI-only. |
| `undo_redo` | `ui_only` | `no` | `yes` | Undo/redo is a TUI interaction feature. |
| `support_bundle` | `workflow` | `yes` | `no` | Support-bundle generation remains a CLI workflow. |
| `completions_and_manpages` | `frontend` | `yes` | `no` | CLI shell completion and manpage generation remain CLI-only deliverables. |
| `hec_ingest` | `read_write` | `yes` | `no` | TUI does not expose HEC ingest flows by product choice. |
