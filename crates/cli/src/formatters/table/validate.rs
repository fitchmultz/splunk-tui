//! SPL validation result table formatter.
//!
//! Responsibilities:
//! - Format validation results as human-readable text.
//!
//! Does NOT handle:
//! - Other output formats.

use anyhow::Result;
use splunk_client::models::ValidateSplResponse;

/// Format validation result as human-readable text.
pub fn format_validation_result(result: &ValidateSplResponse) -> Result<String> {
    let mut output = String::new();

    if result.valid {
        output.push_str("✓ SPL is valid\n");

        if !result.warnings.is_empty() {
            output.push_str("\nWarnings:\n");
            for (i, warning) in result.warnings.iter().enumerate() {
                output.push_str(&format!("  {}. ", i + 1));
                if let (Some(line), Some(col)) = (warning.line, warning.column) {
                    output.push_str(&format!("Line {}, Column {}: ", line, col));
                }
                output.push_str(&warning.message);
                output.push('\n');
            }
        }
    } else {
        output.push_str("✗ SPL has errors\n\n");

        if !result.errors.is_empty() {
            output.push_str("Errors:\n");
            for (i, error) in result.errors.iter().enumerate() {
                output.push_str(&format!("  {}. ", i + 1));
                if let (Some(line), Some(col)) = (error.line, error.column) {
                    output.push_str(&format!("Line {}, Column {}: ", line, col));
                }
                output.push_str(&error.message);
                output.push('\n');
            }
        } else {
            output.push_str("  Unknown error occurred during validation.\n");
        }
    }

    Ok(output)
}
