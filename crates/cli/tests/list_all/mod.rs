//! Integration tests for `splunk-cli list-all` command.
//!
//! This module contains tests for the list-all command, organized by category:
//! - help_tests: Help flag and documentation tests
//! - single_profile_tests: Single-profile mode tests (backward compatibility)
//! - multi_profile_tests: Multi-profile configuration tests
//! - output_format_tests: Output format validation tests
//! - normalization_tests: Input normalization tests (whitespace, case, deduplication)
//! - ordering_tests: Deterministic ordering tests
//!
//! Does NOT:
//! - Test actual Splunk server connectivity (see test-live tests)

pub mod help_tests;
pub mod multi_profile_tests;
pub mod normalization_tests;
pub mod ordering_tests;
pub mod output_format_tests;
pub mod single_profile_tests;
