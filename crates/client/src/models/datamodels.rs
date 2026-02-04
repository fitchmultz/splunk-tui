//! Data model types for Splunk data model API.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

/// Data model information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataModel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub displayName: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub app: String,
    /// Whether the data model is accelerated
    #[serde(rename = "accelerated", default)]
    pub is_accelerated: bool,
    /// JSON representation of the data model (for detailed view)
    #[serde(rename = "eai:data", default)]
    pub json_data: Option<String>,
    /// Last updated timestamp
    #[serde(default)]
    pub updated: Option<String>,
}

/// Data model list response.
#[derive(Debug, Deserialize, Clone)]
pub struct DataModelListResponse {
    pub entry: Vec<DataModelEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataModelEntry {
    pub name: String,
    pub content: DataModel,
}
