//! Purpose: Shared human-readable formatting utilities for CLI and TUI consumers.
//! Responsibilities: Format byte sizes consistently across the workspace.
//! Scope: Presentation helpers that are safe to reuse from multiple frontends.
//! Usage: Import `format_bytes` for the default display contract or
//! `format_bytes_with_precision` when a caller must opt into a different decimal precision.
//! Invariants/Assumptions: Uses binary units (1024 base) and clamps the unit selection to TB.

/// Shared binary-size units used by byte formatting helpers.
const BYTE_UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// Format byte count with the workspace's default human-readable contract.
///
/// This keeps CLI and TUI output aligned so the same Splunk data renders with
/// the same precision regardless of frontend.
pub fn format_bytes(bytes: usize) -> String {
    format_bytes_with_precision(bytes, 1)
}

/// Format byte count with appropriate units (B, KB, MB, GB, TB).
///
/// Uses binary units (1 KB = 1024 B). Byte-sized values always render without
/// decimals, while larger units render with the requested precision.
pub fn format_bytes_with_precision(bytes: usize, precision: usize) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let bytes_f = bytes as f64;
    let exp = (bytes_f.log2() / 1024_f64.log2()).min(BYTE_UNITS.len() as f64 - 1.0) as usize;
    let value = bytes_f / 1024_f64.powi(exp as i32);
    format!(
        "{value:.precision$} {}",
        BYTE_UNITS[exp],
        precision = precision
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_format_bytes_terabytes() {
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.0 TB");
    }

    #[test]
    fn test_format_bytes_with_precision_uses_requested_precision_for_units() {
        assert_eq!(format_bytes_with_precision(1536, 2), "1.50 KB");
        assert_eq!(format_bytes_with_precision(1024 * 1024, 3), "1.000 MB");
    }

    #[test]
    fn test_format_bytes_with_precision_keeps_bytes_as_whole_numbers() {
        assert_eq!(format_bytes_with_precision(512, 2), "512 B");
    }
}
