//! Purpose: Enforce repository contracts introduced by the boundary-cleanup stabilization pass.
//! Responsibilities: Check that build defaults, crate boundaries, install surfaces, docs, and capability-matrix artifacts remain aligned.
//! Scope: Static repository contract validation only; does not execute product behavior.
//! Usage: Runs with `cargo test -p architecture-tests` and the local `make ci` gate.
//! Invariants/Assumptions: The repository root is discoverable from the Cargo workspace and docs are generated from checked-in source files.

use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn cargo_config_does_not_require_sccache() {
    let workspace_root = find_workspace_root();
    let cargo_config = read(workspace_root.join(".cargo/config.toml"));

    assert!(
        !cargo_config.contains("rustc-wrapper"),
        ".cargo/config.toml must not hard-require sccache:\n{cargo_config}"
    );
}

#[test]
fn config_crate_does_not_depend_on_ratatui() {
    let workspace_root = find_workspace_root();
    let cargo_toml = read(workspace_root.join("crates/config/Cargo.toml"));

    assert!(
        !cargo_toml.contains("ratatui"),
        "splunk-config must remain UI-agnostic and may not depend on ratatui"
    );
}

#[test]
fn runtime_install_surfaces_exclude_generate_tui_docs() {
    let workspace_root = find_workspace_root();
    let readme = read(workspace_root.join("README.md"));
    let validation = read(workspace_root.join("docs/validation-checklist.md"));
    let dockerfile = read(workspace_root.join("Dockerfile"));
    let build_mk = read(workspace_root.join("mk/build.mk"));

    assert!(
        !build_mk.contains("generate-tui-docs"),
        "install/build surfaces must not ship the internal generate-tui-docs binary"
    );
    assert!(
        dockerfile.contains("splunk-cli") && dockerfile.contains("splunk-tui"),
        "Dockerfile should still build runtime binaries"
    );
    assert!(
        !dockerfile.contains("generate-tui-docs"),
        "Dockerfile runtime build must exclude generate-tui-docs"
    );
    assert!(
        readme.contains("make install-bins"),
        "README should document install-bins as the supported runtime install path"
    );
    assert!(
        validation.contains("make install-bins"),
        "validation checklist should reference install-bins, not internal tooling"
    );
}

#[test]
fn repository_docs_and_makefiles_do_not_expose_helm_or_kubernetes() {
    let workspace_root = find_workspace_root();
    let files = [
        workspace_root.join("Makefile"),
        workspace_root.join("mk/help.mk"),
        workspace_root.join("mk/docker.mk"),
        workspace_root.join("README.md"),
        workspace_root.join("docs/containers.md"),
        workspace_root.join("docs/usage.md"),
    ];

    for path in files {
        let contents = read(&path).to_lowercase();
        assert!(
            !contains_word(&contents, "helm"),
            "{} still references Helm after the cutover",
            path.display()
        );
        assert!(
            !contains_word(&contents, "kubernetes"),
            "{} still references Kubernetes after the cutover",
            path.display()
        );
    }
}

#[test]
fn client_public_surface_does_not_reexport_telemetry_bootstrap_types() {
    let workspace_root = find_workspace_root();
    let lib_rs = read(workspace_root.join("crates/client/src/lib.rs"));

    assert!(
        !lib_rs.contains("metrics_exporter"),
        "splunk-client lib.rs must not publicly expose telemetry bootstrap modules"
    );
    assert!(
        !lib_rs.contains("TracingGuard") && !lib_rs.contains("MetricsExporter"),
        "splunk-client lib.rs must not publicly reexport telemetry bootstrap types"
    );
}

#[test]
fn capability_matrix_markdown_matches_tsv_source() {
    let workspace_root = find_workspace_root();
    let source = read(workspace_root.join("docs/capability-matrix.tsv"));
    let actual = read(workspace_root.join("docs/capability-matrix.md"));
    let expected = render_capability_matrix(&source);

    assert_eq!(
        normalize_newlines(&actual),
        normalize_newlines(&expected),
        "docs/capability-matrix.md is out of sync with docs/capability-matrix.tsv"
    );
}

fn render_capability_matrix(source: &str) -> String {
    let mut lines = source
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
    let header = lines.next().expect("capability matrix header missing");
    assert_eq!(
        header, "capability\tclient\tcli\ttui\trationale",
        "unexpected capability matrix header"
    );

    let mut out = String::from(
        "# Capability Matrix\n\nThis file is generated from `docs/capability-matrix.tsv`. Update the TSV source first, then keep this document in sync.\n\n| Capability | Client | CLI | TUI | Rationale |\n| --- | --- | --- | --- | --- |\n",
    );

    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 5, "invalid capability-matrix row: {line}");
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | {} |\n",
            fields[0], fields[1], fields[2], fields[3], fields[4]
        ));
    }

    out
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn contains_word(contents: &str, needle: &str) -> bool {
    contents
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .any(|token| token == needle)
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("Failed to read {}: {error}", path.as_ref().display()))
}

fn find_workspace_root() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    let mut dir = current_dir.as_path();
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            return dir.to_path_buf();
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return current_dir,
        }
    }
}
