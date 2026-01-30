//! Lookups table formatter.
//!
//! Responsibilities:
//! - Format lookup tables as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::LookupTable;

/// Format lookup tables as a tab-separated table.
pub fn format_lookups(lookups: &[LookupTable]) -> Result<String> {
    let mut output = String::new();

    if lookups.is_empty() {
        return Ok("No lookup tables found.".to_string());
    }

    // Header
    output.push_str("Name\tFilename\tOwner\tApp\tSharing\tSize\n");

    for lookup in lookups {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            lookup.name,
            lookup.filename,
            lookup.owner,
            lookup.app,
            lookup.sharing,
            format_size(lookup.size)
        ));
    }

    Ok(output)
}

/// Format byte size to human-readable string.
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lookups_empty() {
        let lookups: Vec<LookupTable> = vec![];
        let result = format_lookups(&lookups).unwrap();
        assert_eq!(result, "No lookup tables found.");
    }

    #[test]
    fn test_format_lookups_with_data() {
        let lookups = vec![
            LookupTable {
                name: "test_lookup".to_string(),
                filename: "test.csv".to_string(),
                owner: "admin".to_string(),
                app: "search".to_string(),
                sharing: "app".to_string(),
                size: 1024,
            },
            LookupTable {
                name: "big_lookup".to_string(),
                filename: "big.csv".to_string(),
                owner: "user1".to_string(),
                app: "myapp".to_string(),
                sharing: "global".to_string(),
                size: 5 * 1024 * 1024, // 5 MB
            },
        ];

        let result = format_lookups(&lookups).unwrap();
        assert!(result.contains("Name\tFilename\tOwner\tApp\tSharing\tSize"));
        assert!(result.contains("test_lookup\ttest.csv\tadmin\tsearch\tapp\t1.0 KB"));
        assert!(result.contains("big_lookup\tbig.csv\tuser1\tmyapp\tglobal\t5.0 MB"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0.0 B");
        assert_eq!(format_size(512), "512.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }
}
