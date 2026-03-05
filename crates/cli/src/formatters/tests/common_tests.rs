//! Common tests for formatters - escaping, helpers, output format parsing.

use crate::formatters::{
    OutputFormat,
    common::{escape_csv, escape_xml, format_json_value},
};
use serde_json::json;

#[test]
fn test_output_format_from_str() {
    assert_eq!(
        OutputFormat::from_str("json").unwrap(),
        crate::formatters::OutputFormat::Json
    );
    assert_eq!(
        OutputFormat::from_str("JSON").unwrap(),
        crate::formatters::OutputFormat::Json
    );
    assert_eq!(
        OutputFormat::from_str("csv").unwrap(),
        crate::formatters::OutputFormat::Csv
    );
    assert_eq!(
        OutputFormat::from_str("xml").unwrap(),
        crate::formatters::OutputFormat::Xml
    );
    assert_eq!(
        OutputFormat::from_str("table").unwrap(),
        crate::formatters::OutputFormat::Table
    );
    assert!(OutputFormat::from_str("invalid").is_err());
}

#[test]
fn test_xml_escaping() {
    assert_eq!(escape_xml("test&<>'\""), "test&amp;&lt;&gt;&apos;&quot;");
}

#[test]
fn test_csv_escaping() {
    // No escaping needed for simple strings
    assert_eq!(escape_csv("simple"), "simple");
    // Comma requires quoting
    assert_eq!(escape_csv("hello,world"), "\"hello,world\"");
    // Quote requires doubling and wrapping
    assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
    // Newline requires quoting
    assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
    // Mixed special chars
    assert_eq!(
        escape_csv("value, with \"quotes\"\nand newline"),
        "\"value, with \"\"quotes\"\"\nand newline\""
    );
}

#[test]
fn test_format_json_value() {
    // String values
    assert_eq!(format_json_value(&json!("hello")), "hello");
    // Number values
    assert_eq!(format_json_value(&json!(42)), "42");
    assert_eq!(
        format_json_value(&json!(std::f64::consts::PI)),
        format!("{}", std::f64::consts::PI)
    );
    // Boolean values
    assert_eq!(format_json_value(&json!(true)), "true");
    assert_eq!(format_json_value(&json!(false)), "false");
    // Null values
    assert_eq!(format_json_value(&json!(null)), "");
    // Array values (compact JSON)
    assert_eq!(format_json_value(&json!([1, 2, 3])), "[1,2,3]");
    // Object values (compact JSON)
    assert_eq!(
        format_json_value(&json!({"key": "value"})),
        "{\"key\":\"value\"}"
    );
}

// === RQ-0195: format_json_value edge cases ===

#[test]
fn test_format_json_value_deeply_nested() {
    // Create a deeply nested structure (10 levels)
    let mut value = json!("deep");
    for _ in 0..10 {
        value = json!({"level": value});
    }
    let result = format_json_value(&value);
    // Should serialize without panicking
    assert!(result.contains("deep"));
    assert!(result.starts_with("{"));
}

#[test]
fn test_format_json_value_large_array() {
    // Create an array with many elements
    let arr: Vec<i32> = (0..100).collect();
    let value = json!(arr);
    let result = format_json_value(&value);
    // Should serialize without panicking
    assert!(result.contains("0"));
    assert!(result.contains("99"));
    assert!(result.starts_with("["));
}

#[test]
fn test_format_json_value_mixed_types() {
    let value = json!({
        "string": "text",
        "number": 42,
        "float": 1.23456_f64,
        "bool": true,
        "null": null,
        "array": [1, "two", 3.0, false, null],
        "nested": {"key": "value"}
    });
    let result = format_json_value(&value);
    // Should handle all types
    assert!(result.contains("text"));
    assert!(result.contains("42"));
    assert!(result.contains("1.23456"));
    assert!(result.contains("true"));
    assert!(result.starts_with("{"));
}

#[test]
fn test_format_json_value_empty_structures() {
    // Empty array
    let empty_arr = json!([]);
    assert_eq!(format_json_value(&empty_arr), "[]");

    // Empty object
    let empty_obj = json!({});
    assert_eq!(format_json_value(&empty_obj), "{}");

    // Empty string
    let empty_str = json!("");
    assert_eq!(format_json_value(&empty_str), "");
}
