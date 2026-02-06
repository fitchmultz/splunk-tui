//! ResourceDisplay implementation for App.
//!
//! This module provides a unified display schema for App resources,
//! enabling automatic support for CSV, Table, and XML formatters.

use crate::formatters::ResourceDisplay;
use splunk_client::App;

impl ResourceDisplay for App {
    fn headers(detailed: bool) -> Vec<&'static str> {
        // Standard headers used as fallback
        if detailed {
            vec![
                "Name",
                "Label",
                "Version",
                "Disabled",
                "Author",
                "Description",
            ]
        } else {
            vec!["Name", "Label", "Version", "Disabled", "Author"]
        }
    }

    fn headers_csv(detailed: bool) -> Vec<&'static str> {
        // CSV uses lowercase headers to match existing format
        if detailed {
            vec![
                "name",
                "label",
                "version",
                "disabled",
                "author",
                "description",
            ]
        } else {
            vec!["name", "label", "version", "disabled", "author"]
        }
    }

    fn headers_table(detailed: bool) -> Vec<&'static str> {
        // Table uses UPPERCASE headers to match existing format
        if detailed {
            vec![
                "NAME",
                "LABEL",
                "VERSION",
                "DISABLED",
                "AUTHOR",
                "DESCRIPTION",
            ]
        } else {
            vec!["NAME", "LABEL", "VERSION", "DISABLED", "AUTHOR"]
        }
    }

    fn row_data(&self, detailed: bool) -> Vec<Vec<String>> {
        let row = if detailed {
            vec![
                self.name.clone(),
                self.label.clone().unwrap_or_else(|| "-".to_string()),
                self.version.clone().unwrap_or_else(|| "-".to_string()),
                self.disabled.to_string(),
                self.author.clone().unwrap_or_else(|| "-".to_string()),
                self.description.clone().unwrap_or_else(|| "-".to_string()),
            ]
        } else {
            vec![
                self.name.clone(),
                self.label.clone().unwrap_or_else(|| "-".to_string()),
                self.version.clone().unwrap_or_else(|| "-".to_string()),
                self.disabled.to_string(),
                self.author.clone().unwrap_or_else(|| "-".to_string()),
            ]
        };
        vec![row]
    }

    fn xml_element_name() -> &'static str {
        "app"
    }

    fn xml_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("name", Some(self.name.clone())),
            ("disabled", Some(self.disabled.to_string())),
            ("label", self.label.clone()),
            ("version", self.version.clone()),
            ("author", self.author.clone()),
            ("description", self.description.clone()),
            ("isConfigured", self.is_configured.map(|v| v.to_string())),
            ("isVisible", self.is_visible.map(|v| v.to_string())),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_headers() {
        let headers = App::headers(false);
        assert_eq!(
            headers,
            vec!["Name", "Label", "Version", "Disabled", "Author"]
        );

        let detailed = App::headers(true);
        assert!(detailed.contains(&"Description"));
    }

    #[test]
    fn test_app_headers_csv() {
        let headers = App::headers_csv(false);
        assert_eq!(
            headers,
            vec!["name", "label", "version", "disabled", "author"]
        );

        let detailed = App::headers_csv(true);
        assert!(detailed.contains(&"description"));
    }

    #[test]
    fn test_app_headers_table() {
        let headers = App::headers_table(false);
        assert_eq!(
            headers,
            vec!["NAME", "LABEL", "VERSION", "DISABLED", "AUTHOR"]
        );

        let detailed = App::headers_table(true);
        assert!(detailed.contains(&"DESCRIPTION"));
    }

    #[test]
    fn test_app_xml_element_name() {
        assert_eq!(App::xml_element_name(), "app");
    }

    #[test]
    fn test_app_row_data() {
        let app = App {
            name: "search".to_string(),
            label: Some("Search & Reporting".to_string()),
            version: Some("9.0.0".to_string()),
            is_configured: Some(true),
            is_visible: Some(true),
            disabled: false,
            description: Some("The main search app".to_string()),
            author: Some("Splunk".to_string()),
        };

        let rows = app.row_data(false);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "search");
        assert_eq!(rows[0][1], "Search & Reporting");
        assert_eq!(rows[0][2], "9.0.0");
        assert_eq!(rows[0][3], "false");
        assert_eq!(rows[0][4], "Splunk");
    }

    #[test]
    fn test_app_row_data_detailed() {
        let app = App {
            name: "search".to_string(),
            label: Some("Search & Reporting".to_string()),
            version: Some("9.0.0".to_string()),
            is_configured: Some(true),
            is_visible: Some(true),
            disabled: false,
            description: Some("The main search app".to_string()),
            author: Some("Splunk".to_string()),
        };

        let rows = app.row_data(true);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 6); // Detailed has 6 columns
        assert_eq!(rows[0][5], "The main search app");
    }

    #[test]
    fn test_app_row_data_defaults() {
        let app = App {
            name: "test".to_string(),
            label: None,
            version: None,
            is_configured: None,
            is_visible: None,
            disabled: true,
            description: None,
            author: None,
        };

        let rows = app.row_data(false);
        assert_eq!(rows[0][1], "-"); // label
        assert_eq!(rows[0][2], "-"); // version
        assert_eq!(rows[0][4], "-"); // author
    }

    #[test]
    fn test_app_xml_fields() {
        let app = App {
            name: "search".to_string(),
            label: Some("Search & Reporting".to_string()),
            version: Some("9.0.0".to_string()),
            is_configured: Some(true),
            is_visible: Some(true),
            disabled: false,
            description: Some("The main search app".to_string()),
            author: Some("Splunk".to_string()),
        };

        let fields: Vec<(&str, Option<String>)> = app.xml_fields().into_iter().collect();

        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "name" && v.as_deref() == Some("search"))
        );
        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "label" && v.as_deref() == Some("Search & Reporting"))
        );
        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "disabled" && v.as_deref() == Some("false"))
        );
        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "isConfigured" && v.as_deref() == Some("true"))
        );
    }

    #[test]
    fn test_app_xml_fields_omits_none() {
        let app = App {
            name: "test".to_string(),
            label: None,
            version: None,
            is_configured: None,
            is_visible: None,
            disabled: true,
            description: None,
            author: None,
        };

        let fields: Vec<(&str, Option<String>)> = app.xml_fields().into_iter().collect();

        // name and disabled are always present
        assert!(fields.iter().any(|(k, _)| *k == "name"));
        assert!(fields.iter().any(|(k, _)| *k == "disabled"));
        // optional fields are None
        assert!(fields.iter().any(|(k, v)| *k == "label" && v.is_none()));
        assert!(fields.iter().any(|(k, v)| *k == "version" && v.is_none()));
    }
}
