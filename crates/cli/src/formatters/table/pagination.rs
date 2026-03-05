//! Pagination utilities for table formatters.
//!
//! Responsibilities:
//! - Build pagination footer strings.
//! - Provide generic pagination wrapper for formatters.
//!
//! Does NOT handle:
//! - Resource-specific formatting logic.

use crate::formatters::table::Pagination;

/// Build a pagination footer string.
///
/// - `offset` is zero-based
/// - `page_size` is the requested page size
/// - `total` is optional; when absent, footer omits total/page-count
pub fn build_pagination_footer(p: Pagination, shown: usize) -> Option<String> {
    if p.page_size == 0 {
        // Avoid division by zero; caller should validate for client-side pagination.
        return None;
    }

    // If nothing is shown, caller should usually emit a friendlier message.
    if shown == 0 {
        if let Some(total) = p.total {
            if total == 0 {
                return Some("No results.".to_string());
            }
            if p.offset >= total {
                return Some(format!(
                    "Showing 0 of {} (offset {} out of range)",
                    total, p.offset
                ));
            }
            return Some(format!("Showing 0 of {}", total));
        }
        return Some("No results.".to_string());
    }

    let start = p.offset.saturating_add(1);
    let end = p.offset.saturating_add(shown);
    let page = (p.offset / p.page_size).saturating_add(1);

    if let Some(total) = p.total {
        let total_pages: usize = if total == 0 {
            0
        } else {
            (total.saturating_add(p.page_size).saturating_sub(1)) / p.page_size
        };
        Some(format!(
            "Showing {}-{} of {} (page {} of {})",
            start, end, total, page, total_pages
        ))
    } else {
        Some(format!("Showing {}-{} (page {})", start, end, page))
    }
}

/// Format an empty collection message with offset awareness.
pub fn format_empty_message(resource_name: &str, offset: usize) -> String {
    if offset > 0 {
        format!("No {} found for offset {}.", resource_name, offset)
    } else {
        format!("No {} found.", resource_name)
    }
}
