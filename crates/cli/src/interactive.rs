//! User interaction utilities for the CLI.
//!
//! Responsibilities:
//! - Provide shared helpers for interactive user prompts
//! - Ensure consistent UX patterns across all CLI commands
//! - Handle stdin/stdout interactions safely

use anyhow::Result;
use std::io::Write;

/// Prompt the user for delete confirmation.
///
/// Displays a confirmation prompt asking if the user wants to delete the specified item.
/// Returns `true` if the user confirms (enters 'y' or 'Y'), `false` otherwise.
///
/// # Arguments
/// * `item_name` - The name of the item being deleted
/// * `item_type` - The type of item (e.g., "index", "user", "role") for the prompt message
///
/// # Returns
/// * `Ok(true)` - User confirmed deletion
/// * `Ok(false)` - User cancelled or declined
/// * `Err(...)` - IO error reading from stdin
///
/// # Example
/// ```rust,no_run
/// use splunk_cli::interactive::confirm_delete;
///
/// if confirm_delete("my_index", "index")? {
///     // Proceed with deletion
/// }
/// ```
pub fn confirm_delete(item_name: &str, item_type: &str) -> Result<bool> {
    print!(
        "Are you sure you want to delete {} '{}'? [y/N] ",
        item_type, item_name
    );
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Delete cancelled.");
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    // Note: confirm_delete cannot be easily unit tested due to stdin interaction.
    // Integration tests should cover the happy path and cancellation scenarios.
}
