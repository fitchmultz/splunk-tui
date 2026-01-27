//! Generate TUI keybinding documentation from the centralized keymap.
//!
//! Responsibilities:
//! - Replace the marked keybinding block in docs/usage.md with generated content.
//! - Provide a stable, repeatable output for local CI.
//!
//! Non-responsibilities:
//! - Editing any other documentation sections.
//! - Validating runtime TUI behavior.
//!
//! Invariants:
//! - The markers must exist in the target file.
//! - Generated content is derived from the keymap only.

use std::fs;
use std::path::PathBuf;

use clap::Parser;

/// Generate the TUI keybinding block in docs/usage.md.
#[derive(Debug, Parser)]
#[command(
    name = "generate-tui-docs",
    about = "Regenerate the TUI keybinding section in docs/usage.md",
    after_help = "Examples:\n  generate-tui-docs\n  generate-tui-docs --path docs/usage.md\n"
)]
struct Args {
    /// Path to the usage guide markdown file.
    #[arg(long, default_value = "docs/usage.md")]
    path: PathBuf,
}

const START_MARKER: &str = "<!-- BEGIN TUI KEYBINDINGS -->";
const END_MARKER: &str = "<!-- END TUI KEYBINDINGS -->";

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
    let content = fs::read_to_string(&args.path)?;
    let generated = splunk_tui::render_tui_keybinding_docs();
    let new_content = replace_keybinding_block(&content, &generated)?;

    fs::write(&args.path, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{END_MARKER, START_MARKER, replace_keybinding_block};

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
}
