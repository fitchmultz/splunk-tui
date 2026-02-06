//! Macros for auto-implementing Formatter trait methods.
//!
//! These macros eliminate boilerplate by delegating to the ResourceDisplay trait.
//! Each formatter macro generates the appropriate formatting logic based on
//! the resource type's self-describing schema.
///
/// # Usage
/// ```ignore
/// impl_csv_formatter! {
///     format_users: &[User] => users,
///     format_apps: &[App] => apps,
/// }
/// ```
#[macro_export]
macro_rules! impl_csv_formatter {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource]) -> anyhow::Result<String> {
                use $crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
                use $crate::formatters::ResourceDisplay;

                let mut output = String::new();
                let headers = <$resource as ResourceDisplay>::headers_csv(false);
                let header_strs: Vec<&str> = headers.iter().map(|s| *s).collect();
                output.push_str(&build_csv_header(&header_strs));

                for item in $param {
                    for row in item.row_data_csv(false) {
                        let escaped: Vec<String> = row.iter().map(|v| escape_csv(v)).collect();
                        output.push_str(&build_csv_row(&escaped));
                    }
                }

                Ok(output)
            }
        )*
    };
}

/// Macro to implement CSV formatter methods for ResourceDisplay types (with detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_csv_formatter_detailed! {
///     format_indexes: &[Index] => indexes,
/// }
/// ```
#[macro_export]
macro_rules! impl_csv_formatter_detailed {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource], detailed: bool) -> anyhow::Result<String> {
                use $crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
                use $crate::formatters::ResourceDisplay;

                let mut output = String::new();
                let headers = <$resource as ResourceDisplay>::headers_csv(detailed);
                let header_strs: Vec<&str> = headers.iter().map(|s| *s).collect();
                output.push_str(&build_csv_header(&header_strs));

                for item in $param {
                    for row in item.row_data_csv(detailed) {
                        let escaped: Vec<String> = row.iter().map(|v| escape_csv(v)).collect();
                        output.push_str(&build_csv_row(&escaped));
                    }
                }

                Ok(output)
            }
        )*
    };
}

/// Macro to implement Table formatter methods for ResourceDisplay types (without detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_table_formatter! {
///     format_users: &[User] => users,
///     format_apps: &[App] => apps,
/// }
/// ```
#[macro_export]
macro_rules! impl_table_formatter {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource]) -> anyhow::Result<String> {
                use $crate::formatters::ResourceDisplay;

                if $param.is_empty() {
                    // Handle pluralization for common resource types
                    let resource_name = stringify!($resource);
                    let display_name = match resource_name {
                        "App" => "apps",
                        "User" => "users",
                        "Index" => "indexes",
                        "SavedSearch" => "saved searches",
                        _ => resource_name.to_lowercase().leak(),
                    };
                    return Ok(format!("No {} found.", display_name));
                }

                let mut output = String::new();
                let headers = <$resource as ResourceDisplay>::headers_table(false);

                // Calculate column widths based on headers and data
                let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

                // Pre-collect all row data to calculate widths
                let all_rows: Vec<Vec<String>> = $param
                    .iter()
                    .flat_map(|item| item.row_data_table(false))
                    .collect();

                for row in &all_rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i < widths.len() {
                            widths[i] = widths[i].max(cell.len());
                        }
                    }
                }

                // Ensure minimum widths for readability
                for width in &mut widths {
                    *width = (*width).max(10);
                }

                // Build header row
                for (i, header) in headers.iter().enumerate() {
                    if i > 0 {
                        output.push(' ');
                    }
                    let width = widths.get(i).copied().unwrap_or(10);
                    output.push_str(&format!("{:<width$}", header, width = width));
                }
                output.push('\n');

                // Build separator row
                for (i, width) in widths.iter().enumerate() {
                    if i > 0 {
                        output.push(' ');
                    }
                    output.push_str(&"=".repeat(*width));
                }
                output.push('\n');

                // Build data rows
                for row in all_rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i > 0 {
                            output.push(' ');
                        }
                        let width = widths.get(i).copied().unwrap_or(10);
                        output.push_str(&format!("{:<width$}", cell, width = width));
                    }
                    output.push('\n');
                }

                Ok(output)
            }
        )*
    };
}

/// Macro to implement Table formatter methods for ResourceDisplay types (with detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_table_formatter_detailed! {
///     format_indexes: &[Index] => indexes,
/// }
/// ```
#[macro_export]
macro_rules! impl_table_formatter_detailed {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource], detailed: bool) -> anyhow::Result<String> {
                use $crate::formatters::ResourceDisplay;

                if $param.is_empty() {
                    return Ok(format!("No {} found.\n", stringify!($resource).to_lowercase()));
                }

                let mut output = String::new();
                let headers = <$resource as ResourceDisplay>::headers_table(detailed);

                // Calculate column widths based on headers and data
                let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

                // Pre-collect all row data to calculate widths
                let all_rows: Vec<Vec<String>> = $param
                    .iter()
                    .flat_map(|item| item.row_data_table(detailed))
                    .collect();

                for row in &all_rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i < widths.len() {
                            widths[i] = widths[i].max(cell.len());
                        }
                    }
                }

                // Ensure minimum widths for readability
                for width in &mut widths {
                    *width = (*width).max(10);
                }

                // Build header row
                for (i, header) in headers.iter().enumerate() {
                    if i > 0 {
                        output.push(' ');
                    }
                    let width = widths.get(i).copied().unwrap_or(10);
                    output.push_str(&format!("{:<width$}", header, width = width));
                }
                output.push('\n');

                // Build separator row
                for (i, width) in widths.iter().enumerate() {
                    if i > 0 {
                        output.push(' ');
                    }
                    output.push_str(&"=".repeat(*width));
                }
                output.push('\n');

                // Build data rows
                for row in all_rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i > 0 {
                            output.push(' ');
                        }
                        let width = widths.get(i).copied().unwrap_or(10);
                        output.push_str(&format!("{:<width$}", cell, width = width));
                    }
                    output.push('\n');
                }

                Ok(output)
            }
        )*
    };
}

/// Macro to implement XML list formatter methods for ResourceDisplay types (without detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_xml_list_formatter! {
///     format_users: &[User] => users,
///     format_apps: &[App] => apps,
/// }
/// ```
#[macro_export]
macro_rules! impl_xml_list_formatter {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource]) -> anyhow::Result<String> {
                use $crate::formatters::ResourceDisplay;
                use $crate::formatters::common::escape_xml;

                let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                let elem_name = <$resource as ResourceDisplay>::xml_element_name();
                let plural_name = format!("{}s", elem_name);
                output.push_str(&format!("<{}>\n", plural_name));

                for item in $param {
                    output.push_str(&format!("  <{}>\n", elem_name));
                    for (name, value) in item.xml_fields() {
                        if let Some(v) = value {
                            output.push_str(&format!(
                                "    <{}>{}</{}>\n",
                                name,
                                escape_xml(&v),
                                name
                            ));
                        }
                    }
                    output.push_str(&format!("  </{}>\n", elem_name));
                }

                output.push_str(&format!("</{}>\n", plural_name));
                Ok(output)
            }
        )*
    };
}

/// Macro to implement XML list formatter methods for ResourceDisplay types (with detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_xml_list_formatter_detailed! {
///     format_indexes: &[Index] => indexes,
/// }
/// ```
#[macro_export]
macro_rules! impl_xml_list_formatter_detailed {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &[$resource], _detailed: bool) -> anyhow::Result<String> {
                use $crate::formatters::ResourceDisplay;
                use $crate::formatters::common::escape_xml;

                let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                let elem_name = <$resource as ResourceDisplay>::xml_element_name();
                let plural_name = format!("{}s", elem_name);
                output.push_str(&format!("<{}>\n", plural_name));

                for item in $param {
                    output.push_str(&format!("  <{}>\n", elem_name));
                    for (name, value) in item.xml_fields() {
                        if let Some(v) = value {
                            output.push_str(&format!(
                                "    <{}>{}</{}>\n",
                                name,
                                escape_xml(&v),
                                name
                            ));
                        }
                    }
                    output.push_str(&format!("  </{}>\n", elem_name));
                }

                output.push_str(&format!("</{}>\n", plural_name));
                Ok(output)
            }
        )*
    };
}

/// Macro to implement XML detail formatter methods for single resources.
///
/// # Usage
/// ```ignore
/// impl_xml_detail_formatter! {
///     format_app_info: &App => app,
/// }
/// ```
#[macro_export]
macro_rules! impl_xml_detail_formatter {
    (
        $(
            $method:ident: &$resource:ty => $param:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, $param: &$resource) -> anyhow::Result<String> {
                use $crate::formatters::ResourceDisplay;
                use $crate::formatters::common::escape_xml;

                let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                let elem_name = <$resource as ResourceDisplay>::xml_element_name();
                output.push_str(&format!("<{}>\n", elem_name));

                for (name, value) in $param.xml_fields() {
                    if let Some(v) = value {
                        output.push_str(&format!(
                            "  <{}>{}</{}>\n",
                            name,
                            escape_xml(&v),
                            name
                        ));
                    }
                }

                output.push_str(&format!("</{}>\n", elem_name));
                Ok(output)
            }
        )*
    };
}
