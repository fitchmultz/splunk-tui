//! HEC (HTTP Event Collector) models for Splunk event ingestion.
//!
//! This module provides types for sending events to Splunk via the HEC API.
//! HEC uses a separate endpoint (typically port 8088) and separate authentication
//! (HEC tokens) from the standard Splunk REST API.
//!
//! # What this module handles:
//! - Single event submission with metadata
//! - Batch event submission (JSON array and NDJSON formats)
//! - Health check responses
//! - Acknowledgment status for guaranteed delivery
//!
//! # What this module does NOT handle:
//! - Direct HTTP request implementation (see [`crate::endpoints::hec`])
//! - HEC token management (handled by CLI/config)
//!
//! # Invariants
//! - The `event` field is required and can be any JSON-serializable value
//! - Time can be specified in seconds (with decimals for milliseconds) or milliseconds
//! - HEC responses use a different format than standard Splunk REST API responses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single HEC event to be sent to Splunk.
///
/// The `event` field contains the actual event data and can be any JSON-serializable
/// value. Optional metadata fields (index, source, sourcetype, host, time) can be
/// used to override the defaults configured for the HEC token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecEvent {
    /// The event data (can be any JSON-serializable value).
    /// This is the only required field.
    pub event: serde_json::Value,

    /// Destination index (optional, uses HEC token default if not specified).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,

    /// Source field (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Sourcetype field (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sourcetype: Option<String>,

    /// Host field (optional, defaults to sender IP if not specified).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// Event timestamp in Unix epoch format (seconds or milliseconds).
    /// Can include decimal fractions for sub-second precision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
}

impl HecEvent {
    /// Create a new HEC event with just the event data.
    ///
    /// # Arguments
    /// * `event` - The event data (any JSON-serializable value)
    ///
    /// # Example
    /// ```
    /// use splunk_client::models::HecEvent;
    ///
    /// let event = HecEvent::new(serde_json::json!({"message": "Hello Splunk"}));
    /// ```
    pub fn new(event: serde_json::Value) -> Self {
        Self {
            event,
            index: None,
            source: None,
            sourcetype: None,
            host: None,
            time: None,
        }
    }

    /// Set the destination index.
    pub fn with_index(mut self, index: impl Into<String>) -> Self {
        self.index = Some(index.into());
        self
    }

    /// Set the source field.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the sourcetype field.
    pub fn with_sourcetype(mut self, sourcetype: impl Into<String>) -> Self {
        self.sourcetype = Some(sourcetype.into());
        self
    }

    /// Set the host field.
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the event timestamp.
    pub fn with_time(mut self, time: f64) -> Self {
        self.time = Some(time);
        self
    }
}

/// HEC response for single event submission.
///
/// HEC returns a simple JSON response with a code and text.
/// Code 0 indicates success; non-zero codes indicate errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecResponse {
    /// Response code (0 = success, non-zero = error).
    pub code: i32,

    /// Response text (e.g., "Success" or error message).
    pub text: String,

    /// Acknowledgment ID (when acknowledgments are enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_id: Option<u64>,
}

impl HecResponse {
    /// Check if the response indicates success.
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// Get a human-readable description of the response.
    pub fn description(&self) -> String {
        match self.code {
            0 => "Success".to_string(),
            1 => "Token is required".to_string(),
            2 => "Invalid token".to_string(),
            3 => "Invalid input data format".to_string(),
            4 => "Incorrect index".to_string(),
            5 => "Data channel is missing".to_string(),
            6 => "Event field is required".to_string(),
            7 => "Acknowledgment is disabled".to_string(),
            8 => "Acknowledgment ID not found".to_string(),
            9 => "Internal server error".to_string(),
            10 => "Data channel is disabled".to_string(),
            11 => "Data channel capacity is full".to_string(),
            12 => "Indexer is busy".to_string(),
            13 => "Acknowledgment query is not supported".to_string(),
            14 => "Error in handling indexed fields".to_string(),
            15 => "Error in handling JSON fields".to_string(),
            _ => format!("Unknown error code: {}", self.code),
        }
    }
}

/// HEC batch response (for multiple events).
///
/// When sending batches with acknowledgments enabled, this response contains
/// acknowledgment IDs for each event in the batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecBatchResponse {
    /// Response code (0 = success, non-zero = error).
    pub code: i32,

    /// Response text (e.g., "Success" or error message).
    pub text: String,

    /// Acknowledgment IDs (when acks are enabled, one per event).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_ids: Option<Vec<u64>>,
}

impl HecBatchResponse {
    /// Check if the response indicates success.
    pub fn is_success(&self) -> bool {
        self.code == 0
    }
}

/// HEC health check response.
///
/// The health endpoint returns a simple text response indicating the health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecHealth {
    /// Health status text (e.g., "HEC is healthy").
    pub text: String,

    /// HTTP status code from the response.
    pub code: u16,
}

impl HecHealth {
    /// Check if the health check indicates a healthy status.
    pub fn is_healthy(&self) -> bool {
        self.code == 200 && self.text.to_lowercase().contains("healthy")
    }
}

/// HEC acknowledgment status request.
///
/// Used to query the status of previously sent events when acknowledgments
/// are enabled for guaranteed delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecAckRequest {
    /// List of acknowledgment IDs to check.
    pub ack_ids: Vec<u64>,
}

/// HEC acknowledgment status response.
///
/// Maps acknowledgment IDs to their indexing status. A value of `true` means
/// the event has been successfully indexed; `false` means it's still pending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecAckStatus {
    /// Map of ack_id -> true if indexed, false if pending.
    pub acks: HashMap<u64, bool>,
}

impl HecAckStatus {
    /// Check if all acknowledgments indicate successful indexing.
    pub fn all_indexed(&self) -> bool {
        self.acks.values().all(|&v| v)
    }

    /// Get the list of pending acknowledgment IDs.
    pub fn pending_ids(&self) -> Vec<u64> {
        self.acks
            .iter()
            .filter(|entry| !*entry.1)
            .map(|entry| *entry.0)
            .collect()
    }

    /// Get the list of successfully indexed acknowledgment IDs.
    pub fn indexed_ids(&self) -> Vec<u64> {
        self.acks
            .iter()
            .filter(|entry| *entry.1)
            .map(|entry| *entry.0)
            .collect()
    }
}

/// Parameters for sending a batch of events.
///
/// This is used internally to configure batch sending behavior.
#[derive(Debug, Clone)]
pub struct SendBatchParams {
    /// Events to send.
    pub events: Vec<HecEvent>,

    /// Use newline-delimited JSON format instead of JSON array.
    pub use_ndjson: bool,
}

impl SendBatchParams {
    /// Create new batch parameters with the given events.
    pub fn new(events: Vec<HecEvent>) -> Self {
        Self {
            events,
            use_ndjson: false,
        }
    }

    /// Use NDJSON format instead of JSON array.
    pub fn with_ndjson(mut self) -> Self {
        self.use_ndjson = true;
        self
    }
}

/// HEC error response.
///
/// This represents an error response from the HEC endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecError {
    /// Error code.
    pub code: i32,

    /// Error text.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hec_event_builder() {
        let event = HecEvent::new(serde_json::json!({"message": "test"}))
            .with_index("main")
            .with_source("myapp")
            .with_sourcetype("json")
            .with_host("server01")
            .with_time(1234567890.123);

        assert_eq!(event.event, serde_json::json!({"message": "test"}));
        assert_eq!(event.index, Some("main".to_string()));
        assert_eq!(event.source, Some("myapp".to_string()));
        assert_eq!(event.sourcetype, Some("json".to_string()));
        assert_eq!(event.host, Some("server01".to_string()));
        assert_eq!(event.time, Some(1234567890.123));
    }

    #[test]
    fn test_hec_response_success() {
        let response = HecResponse {
            code: 0,
            text: "Success".to_string(),
            ack_id: Some(123),
        };

        assert!(response.is_success());
        assert_eq!(response.description(), "Success");
    }

    #[test]
    fn test_hec_response_error() {
        let response = HecResponse {
            code: 2,
            text: "Invalid token".to_string(),
            ack_id: None,
        };

        assert!(!response.is_success());
        assert_eq!(response.description(), "Invalid token");
    }

    #[test]
    fn test_hec_health() {
        let healthy = HecHealth {
            text: "HEC is healthy".to_string(),
            code: 200,
        };
        assert!(healthy.is_healthy());

        let unhealthy = HecHealth {
            text: "HEC is not healthy".to_string(),
            code: 503,
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn test_hec_ack_status() {
        let mut acks = HashMap::new();
        acks.insert(1, true);
        acks.insert(2, false);
        acks.insert(3, true);

        let status = HecAckStatus { acks };

        assert!(!status.all_indexed());
        assert_eq!(status.pending_ids(), vec![2]);
        let mut indexed = status.indexed_ids();
        indexed.sort();
        assert_eq!(indexed, vec![1, 3]);
    }

    #[test]
    fn test_hec_event_serialization() {
        let event = HecEvent::new(serde_json::json!({"message": "test"}))
            .with_index("main")
            .with_host("server01");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("main"));
        assert!(json.contains("server01"));

        // Verify optional fields are skipped
        assert!(!json.contains("sourcetype"));
        assert!(!json.contains("source"));
    }
}
