//! Generate TUI keybinding documentation from the centralized keymap.
//!
//! Responsibilities:
//! - Replace the marked keybinding block in documentation files with generated content.
//! - Provide a stable, repeatable output for local CI.
//!
//! Does NOT handle:
//! - Editing any other documentation sections.
//! - Validating runtime TUI behavior.
//!
//! Invariants:
//! - The markers must exist in all target files.
//! - Generated content is derived from the keymap only.

use std::fs;
use std::path::PathBuf;

use clap::Parser;

/// Generate the TUI keybinding block in documentation files.
#[derive(Debug, Parser)]
#[command(
    name = "generate-tui-docs",
    about = "Regenerate the TUI keybinding section in documentation files",
    after_help = "Examples:\n  generate-tui-docs\n  generate-tui-docs --path docs/usage.md\n"
)]
struct Args {
    /// Paths to the markdown files to update. Defaults to README.md and standard docs.
    #[arg(long)]
    path: Vec<PathBuf>,

    /// Check if the files are up to date without writing.
    #[arg(long)]
    check: bool,
}

const START_MARKER: &str = "<!-- BEGIN TUI KEYBINDINGS -->";
const END_MARKER: &str = "<!-- END TUI KEYBINDINGS -->";
const DEFAULT_PATHS: &[&str] = &["README.md", "docs/usage.md", "docs/user-guide.md"];

fn replace_keybinding_block(content: &str, generated: &str) -> anyhow::Result<String> {
    let Some(start_idx) = content.find(START_MARKER) else {
        anyhow::bail!("Start marker not found: {START_MARKER}");
    };
    let Some(end_idx) = content.find(END_MARKER) else {
        anyhow::bail!("End marker not found: {END_MARKER}");
    };
    if end_idx <= start_idx {
        anyhow::bail!("End marker appears before start marker.");
    }

    let before = &content[..start_idx + START_MARKER.len()];
    let after = &content[end_idx..];

    Ok(format!("{}\n\n{}\n{}", before, generated, after))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let paths = if args.path.is_empty() {
        DEFAULT_PATHS.iter().map(PathBuf::from).collect()
    } else {
        args.path
    };

    let generated = splunk_tui::render_tui_keybinding_docs();

    for path in paths {
        let content = fs::read_to_string(&path)?;
        let new_content = replace_keybinding_block(&content, &generated)?;
        ensure_up_to_date(&path, &content, &new_content, args.check)?;
    }

    Ok(())
}

fn ensure_up_to_date(
    path: &std::path::Path,
    original: &str,
    updated: &str,
    check_only: bool,
) -> anyhow::Result<()> {
    if original != updated {
        if check_only {
            anyhow::bail!(
                "Documentation is out of date. Run 'make generate' to update {}",
                path.display()
            );
        }
        fs::write(path, updated)?;
        println!("Updated {}.", path.display());
    } else {
        println!("{} is already up to date.", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{END_MARKER, START_MARKER, ensure_up_to_date, replace_keybinding_block};
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn replaces_marker_block() {
        let content = format!("intro\n{START_MARKER}\nold\n{END_MARKER}\noutro");
        let updated = replace_keybinding_block(&content, "new content").unwrap();

        assert!(updated.contains("new content"));
        assert!(updated.contains(START_MARKER));
        assert!(updated.contains(END_MARKER));
    }

    #[test]
    fn errors_when_start_missing() {
        let content = format!("{END_MARKER}\n");
        let err = replace_keybinding_block(&content, "new").unwrap_err();

        assert!(err.to_string().contains("Start marker not found"));
    }

    #[test]
    fn errors_when_end_missing() {
        let content = format!("{START_MARKER}\n");
        let err = replace_keybinding_block(&content, "new").unwrap_err();

        assert!(err.to_string().contains("End marker not found"));
    }

    #[test]
    fn errors_when_end_before_start() {
        let content = format!("{END_MARKER}\n{START_MARKER}\n");
        let err = replace_keybinding_block(&content, "new").unwrap_err();

        assert!(
            err.to_string()
                .contains("End marker appears before start marker")
        );
    }

    #[test]
    fn check_mode_fails_when_drifted() {
        let path = std::path::Path::new("test.md");
        let original = "old content";
        let updated = "new content";

        let err = ensure_up_to_date(path, original, updated, true).unwrap_err();
        assert!(err.to_string().contains("Documentation is out of date"));
    }

    #[test]
    fn check_mode_passes_when_matching() {
        let path = std::path::Path::new("test.md");
        let original = "same content";
        let updated = "same content";

        ensure_up_to_date(path, original, updated, true).unwrap();
    }

    #[test]
    fn write_mode_updates_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let original = "old";
        let updated = "new";

        ensure_up_to_date(path, original, updated, false).unwrap();

        let saved = fs::read_to_string(path).unwrap();
        assert_eq!(saved, "new");
    }

    #[test]
    fn write_mode_skips_identical() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let content = "same";

        ensure_up_to_date(path, content, content, false).unwrap();
        // Success means no error, and we trust it didn't write if it said so
    }
}
