//! Macros for auto-implementing Formatter trait methods.
//!
//! These macros eliminate boilerplate by delegating to the ResourceDisplay trait.
//! Each formatter macro generates the appropriate formatting logic based on
//! the resource type's self-describing schema.

/// Macro to implement CSV formatter methods for ResourceDisplay types (without detailed parameter).
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

/// Macro to implement paginated Table formatter methods (with detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_table_paginated_detailed! {
///     format_indexes_paginated: &[Index] => indexes, base: format_indexes, resource_name: "indexes",
/// }
/// ```
#[macro_export]
macro_rules! impl_table_paginated_detailed {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident, base: $base:ident, resource_name: $name:expr
        ),*$(,)?
    ) => {
        $(
            pub fn $method(
                &self,
                $param: &[$resource],
                detailed: bool,
                pagination: $crate::formatters::table::Pagination,
            ) -> anyhow::Result<String> {
                use $crate::formatters::table::pagination::{build_pagination_footer, format_empty_message};

                if $param.is_empty() {
                    return Ok(format_empty_message($name, pagination.offset));
                }

                let mut output = self.$base($param, detailed)?;

                if let Some(footer) = build_pagination_footer(pagination, $param.len()) {
                    output.push('\n');
                    output.push_str(&footer);
                    output.push('\n');
                }

                Ok(output)
            }
        )*
    };
}

/// Macro to implement paginated Table formatter methods (without detailed parameter).
///
/// # Usage
/// ```ignore
/// impl_table_paginated! {
///     format_kvstore_collections_paginated: &[KvStoreCollection] => collections, base: format_kvstore_collections, resource_name: "KVStore collections",
/// }
/// ```
#[macro_export]
macro_rules! impl_table_paginated {
    (
        $(
            $method:ident: &[$resource:ty] => $param:ident, base: $base:ident, resource_name: $name:expr
        ),*$(,)?
    ) => {
        $(
            pub fn $method(
                &self,
                $param: &[$resource],
                pagination: $crate::formatters::table::Pagination,
            ) -> anyhow::Result<String> {
                use $crate::formatters::table::pagination::{build_pagination_footer, format_empty_message};

                if $param.is_empty() {
                    return Ok(format_empty_message($name, pagination.offset));
                }

                let mut output = self.$base($param)?;

                if let Some(footer) = build_pagination_footer(pagination, $param.len()) {
                    output.push('\n');
                    output.push_str(&footer);
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

// =============================================================================
// DELEGATED FORMATTER MACROS
// =============================================================================
// These macros eliminate repetitive delegation boilerplate by generating
// trait method implementations that simply forward to submodule functions.
// =============================================================================

/// Macro to implement formatter methods that delegate to submodule functions
/// for slice parameters (e.g., `&[T]`) without detailed parameter.
///
/// # Usage
/// ```ignore
/// impl_delegated_formatter_slice! {
///     format_jobs: &[SearchJobStatus] => jobs::format_jobs,
///     format_users: &[User] => users::format_users,
/// }
/// ```
#[macro_export]
macro_rules! impl_delegated_formatter_slice {
    (
        $(
            $method:ident: &[$param_type:ty] => $module:ident :: $func:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, items: &[$param_type]) -> anyhow::Result<String> {
                $module::$func(items)
            }
        )*
    };
}

/// Macro to implement formatter methods that delegate to submodule functions
/// for slice parameters with `detailed: bool` parameter.
///
/// # Usage
/// ```ignore
/// impl_delegated_formatter_slice_detailed! {
///     format_indexes: &[Index] => indexes::format_indexes,
///     format_forwarders: &[Forwarder] => forwarders::format_forwarders,
/// }
/// ```
#[macro_export]
macro_rules! impl_delegated_formatter_slice_detailed {
    (
        $(
            $method:ident: &[$param_type:ty] => $module:ident :: $func:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, items: &[$param_type], detailed: bool) -> anyhow::Result<String> {
                $module::$func(items, detailed)
            }
        )*
    };
}

/// Macro to implement formatter methods that delegate to submodule functions
/// for single item reference parameters.
///
/// # Usage
/// ```ignore
/// impl_delegated_formatter_single! {
///     format_job_details: &SearchJobStatus => jobs::format_job_details,
///     format_health: &HealthCheckOutput => health::format_health,
/// }
/// ```
#[macro_export]
macro_rules! impl_delegated_formatter_single {
    (
        $(
            $method:ident: &$param_type:ty => $module:ident :: $func:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, item: &$param_type) -> anyhow::Result<String> {
                $module::$func(item)
            }
        )*
    };
}

/// Macro to implement formatter methods that delegate to submodule functions
/// for slice parameters with `is_first: bool` parameter (streaming).
///
/// # Usage
/// ```ignore
/// impl_delegated_formatter_streaming! {
///     format_logs_streaming: &[LogEntry] => logs::format_logs_streaming,
/// }
/// ```
#[macro_export]
macro_rules! impl_delegated_formatter_streaming {
    (
        $(
            $method:ident: &[$param_type:ty] => $module:ident :: $func:ident
        ),*$(,)?
    ) => {
        $(
            fn $method(&self, items: &[$param_type], is_first: bool) -> anyhow::Result<String> {
                $module::$func(items, is_first)
            }
        )*
    };
}

/// Macro to implement formatter methods that return bail errors for unsupported formats.
///
/// # Usage
/// ```ignore
/// impl_unsupported_formatter! {
///     format_cluster_peers => "CSV format not supported for cluster peers. Use JSON format.",
///     format_shc_members => "CSV format not supported for SHC members. Use JSON format.",
/// }
/// ```
#[macro_export]
macro_rules! impl_unsupported_formatter {
    (
        $(
            $method:ident => $msg:expr
        ),*$(,)?
    ) => {
        $(
            fn $method(
                &self,
                _arg1: impl std::any::Any,
                _pagination: &$crate::formatters::Pagination,
            ) -> anyhow::Result<String> {
                anyhow::bail!($msg)
            }
        )*
    };
}
