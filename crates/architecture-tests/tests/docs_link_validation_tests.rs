//! Purpose: Enforce markdown link integrity for public-facing repository documentation.
//! Responsibilities: Validate that local relative markdown links resolve to existing paths.
//! Scope: README/docs/examples/contributing/security/action docs links; external URLs are ignored.
//! Usage: Runs as part of `cargo test -p architecture-tests` and `make ci`.
//! Invariants/Assumptions: Relative links must resolve from the source file location.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[test]
fn markdown_relative_links_resolve() {
    let workspace_root = find_workspace_root();

    let mut markdown_files = vec![
        workspace_root.join("README.md"),
        workspace_root.join("CONTRIBUTING.md"),
        workspace_root.join("SECURITY.md"),
        workspace_root.join(".github/actions/README.md"),
    ];

    markdown_files.extend(find_markdown_files(&workspace_root.join("docs")));
    markdown_files.extend(find_markdown_files(&workspace_root.join("examples")));

    let mut broken = Vec::new();

    for file in markdown_files {
        if !file.exists() {
            continue;
        }

        let content = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("Failed reading markdown file {:?}: {}", file, e));

        let parent = file.parent().unwrap_or(&workspace_root);
        for target in extract_markdown_link_targets(&content) {
            if should_skip_link(&target) {
                continue;
            }

            let path_part = target
                .split_once('#')
                .map(|(path, _)| path)
                .unwrap_or(target.as_str())
                .trim_matches('<')
                .trim_matches('>')
                .trim();

            if path_part.is_empty() {
                continue;
            }

            let resolved = if path_part.starts_with('/') {
                workspace_root.join(path_part.trim_start_matches('/'))
            } else {
                parent.join(path_part)
            };

            if !resolved.exists() {
                broken.push(format!(
                    "{} -> {} (resolved: {})",
                    relative_to_workspace(&file, &workspace_root),
                    target,
                    resolved.display()
                ));
            }
        }
    }

    assert!(
        broken.is_empty(),
        "Found broken markdown links:\n{}",
        broken.join("\n")
    );
}

fn should_skip_link(target: &str) -> bool {
    let normalized = target.trim();
    normalized.starts_with('#')
        || normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("mailto:")
        || normalized.starts_with("tel:")
        || normalized.starts_with("data:")
}

fn find_markdown_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }

    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

fn extract_markdown_link_targets(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut in_code_fence = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            continue;
        }

        let mut remaining = line;
        while let Some(index) = remaining.find("](") {
            let after = &remaining[index + 2..];
            let Some(close) = after.find(')') else {
                break;
            };

            let target = after[..close].trim();
            if !target.is_empty() {
                links.push(target.to_string());
            }

            remaining = &after[close + 1..];
        }
    }

    links
}

fn relative_to_workspace(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
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
