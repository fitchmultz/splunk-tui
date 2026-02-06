//! Apps XML formatter.
//!
//! This module previously contained manual XML formatting functions for apps.
//! Both `format_apps` and `format_app_info` have been replaced by the
//! `impl_xml_list_formatter!` and `impl_xml_detail_formatter!` macros
//! which use the `ResourceDisplay` trait.
//!
//! This file is kept as a placeholder for any future app-specific XML formatting
//! that may need to be added.

// Module intentionally left minimal - all XML formatting is now handled
// by the ResourceDisplay trait and macros.

#[cfg(test)]
mod tests {
    use splunk_client::App;

    #[test]
    fn test_app_struct_accessible() {
        // Sanity check that the App struct is accessible
        let _app = App {
            name: "test".to_string(),
            label: None,
            version: None,
            is_configured: None,
            is_visible: None,
            disabled: false,
            description: None,
            author: None,
        };
    }
}
