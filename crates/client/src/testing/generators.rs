//! Test data generators using the fake crate.
//!
//! Provides configurable generators for realistic Splunk data including
//! search results, SPL queries, cluster topologies, log entries, and entities.

use fake::Fake;
use fake::faker::chrono::en::DateTime;
use fake::faker::internet::en::{IPv4, Username};
use fake::faker::lorem::en::{Sentence, Word};
use fake::faker::name::en::Name;
use fake::faker::number::en::Digit;
use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::Value;
use std::collections::HashMap;

/// Configuration for null value handling in generated data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullHandling {
    /// No null values generated
    None,
    /// Sparse nulls (5% chance per field)
    Sparse,
    /// Moderate nulls (15% chance per field)
    Moderate,
    /// Dense nulls (30% chance per field)
    Dense,
    /// Custom percentage (0-100)
    Percent(u8),
}

impl NullHandling {
    /// Returns true if a field should be null based on the configured probability.
    fn should_be_null<R: Rng>(&self, rng: &mut R) -> bool {
        let probability = match self {
            NullHandling::None => 0,
            NullHandling::Sparse => 5,
            NullHandling::Moderate => 15,
            NullHandling::Dense => 30,
            NullHandling::Percent(p) => (*p).min(100),
        };
        rng.gen_ratio(probability as u32, 100)
    }
}

// =============================================================================
// Search Results Generator
// =============================================================================

/// Generates realistic Splunk search result data.
///
/// # Example
/// ```ignore
/// use splunk_client::testing::generators::SearchResultsGenerator;
///
/// let generator = SearchResultsGenerator::new()
///     .with_row_count(1000)
///     .with_column_count(10)
///     .with_time_range("-24h", "now")
///     .with_null_handling(NullHandling::Sparse);
///
/// let results = generator.generate();
/// ```
#[derive(Debug, Clone)]
pub struct SearchResultsGenerator {
    row_count: usize,
    column_count: usize,
    start_time: Option<String>,
    end_time: Option<String>,
    null_handling: NullHandling,
    include_raw: bool,
    sourcetypes: Vec<String>,
    indexes: Vec<String>,
}

impl Default for SearchResultsGenerator {
    fn default() -> Self {
        Self {
            row_count: 100,
            column_count: 5,
            start_time: None,
            end_time: None,
            null_handling: NullHandling::None,
            include_raw: true,
            sourcetypes: vec![
                "access_combined".to_string(),
                "syslog".to_string(),
                "json".to_string(),
                "wineventlog".to_string(),
            ],
            indexes: vec![
                "main".to_string(),
                "_internal".to_string(),
                "_audit".to_string(),
            ],
        }
    }
}

impl SearchResultsGenerator {
    /// Create a new generator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of rows to generate.
    pub fn with_row_count(mut self, count: usize) -> Self {
        self.row_count = count;
        self
    }

    /// Set the number of columns (fields) to generate.
    pub fn with_column_count(mut self, count: usize) -> Self {
        self.column_count = count;
        self
    }

    /// Set the time range for generated events.
    /// Use Splunk time modifiers like "-24h", "-7d", "now".
    pub fn with_time_range(mut self, start: &str, end: &str) -> Self {
        self.start_time = Some(start.to_string());
        self.end_time = Some(end.to_string());
        self
    }

    /// Configure null value handling.
    pub fn with_null_handling(mut self, handling: NullHandling) -> Self {
        self.null_handling = handling;
        self
    }

    /// Whether to include the _raw field.
    pub fn with_raw_field(mut self, include: bool) -> Self {
        self.include_raw = include;
        self
    }

    /// Set custom sourcetypes for the data.
    pub fn with_sourcetypes(mut self, sourcetypes: Vec<String>) -> Self {
        self.sourcetypes = sourcetypes;
        self
    }

    /// Set custom indexes for the data.
    pub fn with_indexes(mut self, indexes: Vec<String>) -> Self {
        self.indexes = indexes;
        self
    }

    /// Generate search results as a JSON Value.
    pub fn generate(&self) -> Value {
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        let mut results = Vec::with_capacity(self.row_count);

        // Generate column names
        let standard_fields = ["_time", "_raw", "_sourcetype", "_index", "_host", "_source"];
        let custom_fields: Vec<String> =
            (0..self.column_count.saturating_sub(standard_fields.len()))
                .map(|i| format!("field{}", i + 1))
                .collect();

        for i in 0..self.row_count {
            let mut row = serde_json::Map::new();

            // _time: ISO 8601 timestamp
            let timestamp: String = DateTime().fake();
            row.insert("_time".to_string(), Value::String(timestamp));

            // _raw: Log message
            if self.include_raw {
                let raw: String = Sentence(3..8).fake();
                row.insert("_raw".to_string(), Value::String(raw));
            }

            // _sourcetype
            if let Some(st) = self.sourcetypes.choose(&mut rng) {
                row.insert("_sourcetype".to_string(), Value::String(st.clone()));
            }

            // _index
            if let Some(idx) = self.indexes.choose(&mut rng) {
                row.insert("_index".to_string(), Value::String(idx.clone()));
            }

            // _host
            let host: String = Word().fake();
            row.insert(
                "_host".to_string(),
                Value::String(format!("{}-server-{:03}", host, i % 100)),
            );

            // _source
            row.insert(
                "_source".to_string(),
                Value::String(format!(
                    "/var/log/{}.log",
                    Word().fake::<String>().to_lowercase()
                )),
            );

            // Custom fields
            for field in &custom_fields {
                if !self.null_handling.should_be_null(&mut rng) {
                    let value = match rng.gen_range(0..4) {
                        0 => Value::String(Word().fake()),
                        1 => Value::Number(rng.gen_range(0..10000).into()),
                        2 => Value::Bool(rand::random()),
                        _ => Value::String(Digit().fake()),
                    };
                    row.insert(field.clone(), value);
                }
            }

            results.push(Value::Object(row));
        }

        serde_json::json!({
            "results": results,
            "preview": false,
            "offset": 0,
            "total": self.row_count
        })
    }

    /// Generate and return as a pretty-printed JSON string.
    pub fn generate_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.generate()).expect("Failed to serialize generated data")
    }
}

// =============================================================================
// SPL Query Generator
// =============================================================================

/// Modes for SPL query generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplQueryMode {
    /// Generate valid SPL queries
    Valid,
    /// Generate syntactically invalid queries
    Invalid,
    /// Generate edge-case queries (empty, very long, special characters)
    EdgeCase,
    /// Mix of all modes
    Mixed,
}

/// Generates SPL (Search Processing Language) queries.
///
/// # Example
/// ```ignore
/// use splunk_client::testing::generators::{SplQueryGenerator, SplQueryMode};
///
/// let generator = SplQueryGenerator::new()
///     .with_mode(SplQueryMode::Valid)
///     .with_pipeline_depth(3);
///
/// let query = generator.generate_one();
/// // "index=main sourcetype=access_combined | stats count by host | sort -count"
/// ```
#[derive(Debug, Clone)]
pub struct SplQueryGenerator {
    mode: SplQueryMode,
    pipeline_depth: usize,
    indexes: Vec<String>,
    sourcetypes: Vec<String>,
    _fields: Vec<String>,
}

impl Default for SplQueryGenerator {
    fn default() -> Self {
        Self {
            mode: SplQueryMode::Valid,
            pipeline_depth: 2,
            indexes: vec![
                "main".to_string(),
                "_internal".to_string(),
                "_audit".to_string(),
            ],
            sourcetypes: vec![
                "access_combined".to_string(),
                "syslog".to_string(),
                "json".to_string(),
                "wineventlog".to_string(),
                "aws:cloudtrail".to_string(),
            ],
            _fields: vec![
                "host".to_string(),
                "source".to_string(),
                "sourcetype".to_string(),
                "action".to_string(),
                "status".to_string(),
                "user".to_string(),
                "ip".to_string(),
            ],
        }
    }
}

impl SplQueryGenerator {
    /// Create a new generator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the query generation mode.
    pub fn with_mode(mut self, mode: SplQueryMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the pipeline depth (number of pipe commands).
    pub fn with_pipeline_depth(mut self, depth: usize) -> Self {
        self.pipeline_depth = depth.clamp(1, 10);
        self
    }

    /// Generate a single SPL query.
    pub fn generate_one(&self) -> String {
        match self.mode {
            SplQueryMode::Valid => self.generate_valid(),
            SplQueryMode::Invalid => self.generate_invalid(),
            SplQueryMode::EdgeCase => self.generate_edge_case(),
            SplQueryMode::Mixed => {
                let mut rng = rand::thread_rng();
                match rng.gen_range(0..3) {
                    0 => self.generate_valid(),
                    1 => self.generate_invalid(),
                    _ => self.generate_edge_case(),
                }
            }
        }
    }

    /// Generate multiple queries.
    pub fn generate_many(&self, count: usize) -> Vec<String> {
        (0..count).map(|_| self.generate_one()).collect()
    }

    fn generate_valid(&self) -> String {
        let mut rng = rand::thread_rng();

        // Base search
        let index = self.indexes.choose(&mut rng).unwrap();
        let sourcetype = self.sourcetypes.choose(&mut rng).unwrap();
        let mut query = format!("index={} sourcetype={}", index, sourcetype);

        // Add optional time range (30% chance)
        if rng.gen_ratio(3, 10) {
            let ranges = [
                "earliest=-1h",
                "earliest=-24h",
                "earliest=-7d",
                "earliest=@d latest=now",
            ];
            query.push(' ');
            query.push_str(ranges.choose(&mut rng).unwrap());
        }

        // Add pipeline commands
        let commands = [
            (" | stats count", 0.8),
            (" | stats count by host", 0.7),
            (" | sort -count", 0.5),
            (" | head 100", 0.6),
            (" | eval status=if(count>1000, \"high\", \"low\")", 0.3),
            (" | where count > 10", 0.4),
            (
                " | rex field=_raw \"(?<ip>\\d+\\.\\d+\\.\\d+\\.\\d+)\"",
                0.2,
            ),
            (" | timechart span=1h count", 0.5),
            (" | top limit=10 host", 0.4),
            (" | rare limit=20 source", 0.3),
        ];

        for _ in 0..self.pipeline_depth {
            for (cmd, prob) in &commands {
                if rng.gen_ratio((*prob * 100.0) as u32, 100) {
                    query.push_str(cmd);
                    break;
                }
            }
        }

        query
    }

    fn generate_invalid(&self) -> String {
        let errors = [
            "index=",                         // Missing value
            "| stats",                        // Missing base search
            "index=main | invalidcommand",    // Invalid command
            "index=main | stats by",          // Missing function
            "index=main | eval ",             // Incomplete eval
            "index=main | where ",            // Incomplete where
            "index=main | stats count(count", // Mismatched parens
            "index=main | | stats count",     // Double pipe
            "index=main sourcetype=",         // Empty sourcetype
        ];
        errors[rand::random::<usize>() % errors.len()].to_string()
    }

    fn generate_edge_case(&self) -> String {
        let edge_cases = [
            "".to_string(),
            " ".to_string(),
            "index=main".to_string(),
            (0..100).map(|_| "| stats count ").collect::<String>(),
            "index=main | stats count | eval a=\"\\\\\"".to_string(),
            "index=main | search \"very long search string ".to_string() + &"x".repeat(10000),
            "index=main | eval special=\"!@#$%^&*()\"".to_string(),
            "index=мейн".to_string(), // Unicode
        ];
        edge_cases[rand::random::<usize>() % edge_cases.len()].clone()
    }
}

// =============================================================================
// Cluster Topology Generator
// =============================================================================

/// Generates realistic Splunk cluster topologies.
///
/// # Example
/// ```ignore
/// use splunk_client::testing::generators::ClusterTopologyGenerator;
///
/// let topology = ClusterTopologyGenerator::new()
///     .with_manager_count(1)
///     .with_peer_count(5)
///     .with_site_count(2)
///     .generate();
/// ```
#[derive(Debug, Clone)]
pub struct ClusterTopologyGenerator {
    manager_count: usize,
    peer_count: usize,
    site_count: usize,
    include_search_heads: bool,
    replication_factor: u32,
    search_factor: u32,
}

impl Default for ClusterTopologyGenerator {
    fn default() -> Self {
        Self {
            manager_count: 1,
            peer_count: 3,
            site_count: 1,
            include_search_heads: true,
            replication_factor: 3,
            search_factor: 2,
        }
    }
}

impl ClusterTopologyGenerator {
    /// Create a new generator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of cluster managers.
    pub fn with_manager_count(mut self, count: usize) -> Self {
        self.manager_count = count.max(1);
        self
    }

    /// Set the number of indexer peers.
    pub fn with_peer_count(mut self, count: usize) -> Self {
        self.peer_count = count;
        self
    }

    /// Set the number of sites (for multisite clusters).
    pub fn with_site_count(mut self, count: usize) -> Self {
        self.site_count = count.max(1);
        self
    }

    /// Whether to include search heads in the topology.
    pub fn with_search_heads(mut self, include: bool) -> Self {
        self.include_search_heads = include;
        self
    }

    /// Set the replication factor.
    pub fn with_replication_factor(mut self, rf: u32) -> Self {
        self.replication_factor = rf;
        self
    }

    /// Set the search factor.
    pub fn with_search_factor(mut self, sf: u32) -> Self {
        self.search_factor = sf;
        self
    }

    /// Generate cluster topology as JSON.
    pub fn generate(&self) -> Value {
        let mut rng = rand::thread_rng();
        let mut peers = Vec::new();

        for i in 0..self.peer_count {
            let site = if self.site_count > 1 {
                format!("site{}", (i % self.site_count) + 1)
            } else {
                "site1".to_string()
            };

            let status = match rng.gen_range(0..10) {
                0 => "Down",
                1..=8 => "Up",
                _ => "Pending",
            };

            let peer_state = match rng.gen_range(0..5) {
                0 => "Searchable",
                1 => "Unsearchable",
                2 => "Streaming",
                _ => "Searchable",
            };

            peers.push(serde_json::json!({
                "id": format!("peer-{}", i + 1),
                "label": format!("Peer Node {}", i + 1),
                "status": status,
                "peer_state": peer_state,
                "site": site,
                "guid": format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
                    rand::random::<u32>(), rand::random::<u16>(), rand::random::<u16>(),
                    rand::random::<u16>(), rand::random::<u64>()),
                "host": format!("splunk-peer-{:02}.example.com", i + 1),
                "port": 8089,
                "replication_count": rng.gen_range(100..10000),
                "replication_status": if rng.gen_bool(0.9) { "Complete" } else { "Pending" },
                "bundle_replication_count": rng.gen_range(10..100),
                "is_captain": i == 0 && self.peer_count > 0,
            }));
        }

        let manager = serde_json::json!({
            "id": "manager-01",
            "label": "Cluster Manager",
            "mode": "manager",
            "status": "enabled",
            "manager_uri": format!("https://{}:8089", IPv4().fake::<String>()),
            "replication_factor": self.replication_factor,
            "search_factor": self.search_factor,
        });

        let mut topology = serde_json::json!({
            "cluster_manager": manager,
            "peers": peers,
            "metadata": {
                "total_peers": self.peer_count,
                "healthy_peers": peers.iter().filter(|p| p["status"] == "Up").count(),
                "sites": self.site_count,
            }
        });

        // Add search heads if requested
        if self.include_search_heads {
            let mut search_heads = Vec::new();
            let sh_count = rng.gen_range(1..=3);
            for i in 0..sh_count {
                search_heads.push(serde_json::json!({
                    "id": format!("search-head-{}", i + 1),
                    "label": format!("Search Head {}", i + 1),
                    "host": format!("splunk-sh-{:02}.example.com", i + 1),
                    "port": 8089,
                    "status": if rng.gen_bool(0.95) { "Up" } else { "Down" },
                }));
            }
            if let Some(obj) = topology.as_object_mut() {
                obj.insert("search_heads".to_string(), Value::Array(search_heads));
            }
        }

        topology
    }
}

// =============================================================================
// Log Entry Generator
// =============================================================================

/// Time format options for log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogTimeFormat {
    /// ISO 8601 format
    Iso8601,
    /// Unix timestamp (seconds)
    UnixTimestamp,
    /// Splunk default format
    SplunkDefault,
}

/// Generates realistic Splunk internal log entries.
///
/// # Example
/// ```ignore
/// use splunk_client::testing::generators::LogEntryGenerator;
///
/// let generator = LogEntryGenerator::new()
///     .with_component("Metrics")
///     .with_level("INFO");
///
/// let logs = generator.generate_many(100);
/// ```
#[derive(Debug, Clone)]
pub struct LogEntryGenerator {
    component: Option<String>,
    level: Option<String>,
    time_format: LogTimeFormat,
}

impl Default for LogEntryGenerator {
    fn default() -> Self {
        Self {
            component: None,
            level: None,
            time_format: LogTimeFormat::Iso8601,
        }
    }
}

impl LogEntryGenerator {
    /// Create a new generator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a specific component name.
    pub fn with_component(mut self, component: &str) -> Self {
        self.component = Some(component.to_string());
        self
    }

    /// Set a specific log level.
    pub fn with_level(mut self, level: &str) -> Self {
        self.level = Some(level.to_string());
        self
    }

    /// Set the time format.
    pub fn with_time_format(mut self, format: LogTimeFormat) -> Self {
        self.time_format = format;
        self
    }

    /// Generate a single log entry.
    pub fn generate_one(&self) -> Value {
        let mut rng = rand::thread_rng();

        let levels = ["ERROR", "WARN", "INFO", "DEBUG", "FATAL"];
        let components = [
            "Metrics",
            "DateParserVerbose",
            "Aggregator",
            "SearchParser",
            "Indexer",
            "Forwarder",
            "LicenseManager",
            "ClusterManager",
            "BucketMover",
            "CMBundlePush",
            "ReplicationManager",
        ];

        let level = self
            .level
            .as_deref()
            .unwrap_or_else(|| levels.choose(&mut rng).unwrap());

        let component = self
            .component
            .as_deref()
            .unwrap_or_else(|| components.choose(&mut rng).unwrap());

        let time = match self.time_format {
            LogTimeFormat::Iso8601 => DateTime().fake::<String>(),
            LogTimeFormat::UnixTimestamp => {
                format!("{}", (1_600_000_000i64..2_000_000_000).fake::<i64>())
            }
            LogTimeFormat::SplunkDefault => DateTime().fake::<String>(),
        };

        let messages: Vec<String> = match level {
            "ERROR" => vec![
                "Failed to connect to indexer".to_string(),
                "License violation detected".to_string(),
                "Disk space critically low".to_string(),
            ],
            "WARN" => vec![
                "Search peer not responding".to_string(),
                "Replication lag detected".to_string(),
                " nearing license limit".to_string(),
            ],
            _ => vec![
                Sentence(5..10).fake(),
                "Operation completed successfully".to_string(),
                "Metrics collection completed".to_string(),
            ],
        };

        let message = messages.choose(&mut rng).unwrap().clone();

        serde_json::json!({
            "_time": time,
            "_indextime": DateTime().fake::<String>(),
            "_serial": format!("{}", rng.gen_range(0..1000000)),
            "log_level": level,
            "component": component,
            "_raw": message,
        })
    }

    /// Generate multiple log entries.
    pub fn generate_many(&self, count: usize) -> Value {
        let entries: Vec<Value> = (0..count).map(|_| self.generate_one()).collect();
        serde_json::json!({ "results": entries })
    }
}

// =============================================================================
// Entity Generators (User, App, Index)
// =============================================================================

/// Generates Splunk user entities.
#[derive(Debug, Clone, Default)]
pub struct UserGenerator;

impl UserGenerator {
    /// Create a new user generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a single user.
    pub fn generate_one(&self) -> Value {
        let mut rng = rand::thread_rng();

        let name: String = Username().fake();
        let roles = ["admin", "power", "user", "can_delete"];
        let user_types = ["Splunk", "SSO", "LDAP", "SAML"];

        serde_json::json!({
            "name": name,
            "realname": Name().fake::<String>(),
            "email": format!("{}@example.com", name),
            "type": user_types.choose(&mut rng).unwrap(),
            "defaultApp": "search",
            "roles": vec![roles.choose(&mut rng).unwrap()],
            "lastSuccessfulLogin": rng.gen_range(1_700_000_000i64..1_800_000_000i64),
        })
    }

    /// Generate multiple users.
    pub fn generate_many(&self, count: usize) -> Value {
        let users: Vec<Value> = (0..count).map(|_| self.generate_one()).collect();
        serde_json::json!({ "entry": users })
    }
}

/// Generates Splunk app entities.
#[derive(Debug, Clone, Default)]
pub struct AppGenerator;

impl AppGenerator {
    /// Create a new app generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a single app.
    pub fn generate_one(&self) -> Value {
        let mut rng = rand::thread_rng();

        let name: String = Word().fake();
        let version = format!(
            "{}.{}.{}",
            rng.gen_range(1..10),
            rng.gen_range(0..20),
            rng.gen_range(0..10)
        );

        serde_json::json!({
            "name": name.to_lowercase(),
            "label": format!("{} App", name.chars().next().unwrap().to_uppercase().to_string() + &name[1..]),
            "version": version,
            "isConfigured": rng.gen_bool(0.8),
            "isVisible": rng.gen_bool(0.9),
            "disabled": rng.gen_bool(0.1),
            "description": Sentence(5..15).fake::<String>(),
            "author": "Splunk Inc.",
        })
    }

    /// Generate multiple apps.
    pub fn generate_many(&self, count: usize) -> Value {
        let apps: Vec<Value> = (0..count).map(|_| self.generate_one()).collect();
        serde_json::json!({ "entry": apps })
    }
}

/// Generates Splunk index entities.
#[derive(Debug, Clone)]
pub struct IndexGenerator {
    include_internal: bool,
}

impl Default for IndexGenerator {
    fn default() -> Self {
        Self {
            include_internal: true,
        }
    }
}

impl IndexGenerator {
    /// Create a new index generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Include internal indexes in generation.
    pub fn with_internal(mut self, include: bool) -> Self {
        self.include_internal = include;
        self
    }

    /// Generate a single index.
    pub fn generate_one(&self) -> Value {
        let mut rng = rand::thread_rng();

        let names = if self.include_internal {
            vec![
                "main",
                "_internal",
                "_audit",
                "_introspection",
                "summary",
                "history",
            ]
        } else {
            vec!["main", "summary"]
        };

        let name = names.choose(&mut rng).unwrap();

        serde_json::json!({
            "name": name,
            "maxTotalDataSizeMB": rng.gen_range(100000..10000000),
            "currentDBSizeMB": rng.gen_range(1000..500000),
            "totalEventCount": rng.gen_range(100000..1000000000i64),
            "maxWarmDBCount": rng.gen_range(100..1000),
            "maxHotBuckets": "auto",
            "frozenTimePeriodInSecs": rng.gen_range(86400..31536000),
            "coldDBPath": format!("/opt/splunk/colddb/{}", name),
            "homePath": format!("/opt/splunk/var/lib/splunk/{}/db", name),
            "thawedPath": format!("/opt/splunk/var/lib/splunk/{}/thaweddb", name),
            "coldToFrozenDir": format!("/opt/splunk/frozen/{}", name),
            "primaryIndex": *name == "main",
        })
    }

    /// Generate multiple indexes.
    pub fn generate_many(&self, count: usize) -> Value {
        let indexes: Vec<Value> = (0..count).map(|_| self.generate_one()).collect();
        serde_json::json!({ "entry": indexes })
    }
}

// =============================================================================
// Bulk Generator Helper
// =============================================================================

/// Bulk generator that can generate multiple entity types.
#[derive(Debug, Clone, Default)]
pub struct BulkGenerator;

impl BulkGenerator {
    /// Create a new bulk generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a complete test dataset with all entity types.
    pub fn generate_dataset(
        &self,
        users: usize,
        apps: usize,
        indexes: usize,
        jobs: usize,
        results: usize,
    ) -> HashMap<String, Value> {
        let mut dataset = HashMap::new();

        dataset.insert(
            "users".to_string(),
            UserGenerator::new().generate_many(users),
        );
        dataset.insert("apps".to_string(), AppGenerator::new().generate_many(apps));
        dataset.insert(
            "indexes".to_string(),
            IndexGenerator::new().generate_many(indexes),
        );
        dataset.insert(
            "jobs".to_string(),
            SearchResultsGenerator::new()
                .with_row_count(results)
                .generate(),
        );
        dataset.insert(
            "logs".to_string(),
            LogEntryGenerator::new().generate_many(jobs),
        );

        dataset
    }
}

// =============================================================================
// Proptest Integration
// =============================================================================

/// Wrap fake-based generators for use with proptest.
///
/// This module provides strategy functions that use fake generators
/// internally, allowing seamless integration with existing proptest tests.
#[cfg(feature = "test-utils")]
pub mod proptest_strategies {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating LogEntry values using fake.
    pub fn fake_log_entry_strategy() -> impl Strategy<Value = Value> {
        let generator = LogEntryGenerator::new();
        (0..1000usize).prop_map(move |_| generator.generate_one())
    }

    /// Strategy for generating User values using fake.
    pub fn fake_user_strategy() -> impl Strategy<Value = Value> {
        let generator = UserGenerator::new();
        (0..1000usize).prop_map(move |_| generator.generate_one())
    }

    /// Strategy for generating SPL queries.
    pub fn spl_query_strategy(mode: super::SplQueryMode) -> impl Strategy<Value = String> {
        let generator = SplQueryGenerator::new().with_mode(mode);
        (0..1000usize).prop_map(move |_| generator.generate_one())
    }

    /// Strategy for generating cluster topologies.
    pub fn cluster_topology_strategy() -> impl Strategy<Value = Value> {
        let generator = ClusterTopologyGenerator::new();
        (0..1000usize).prop_map(move |_| generator.generate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_results_generator() {
        let generator = SearchResultsGenerator::new()
            .with_row_count(10)
            .with_column_count(3);

        let results = generator.generate();
        assert_eq!(results["results"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn test_spl_query_generator_valid() {
        let generator = SplQueryGenerator::new().with_mode(SplQueryMode::Valid);
        let query = generator.generate_one();
        assert!(!query.is_empty());
        assert!(query.starts_with("index="));
    }

    #[test]
    fn test_spl_query_generator_invalid() {
        let generator = SplQueryGenerator::new().with_mode(SplQueryMode::Invalid);
        let query = generator.generate_one();
        // Invalid queries might be empty or have errors
        assert!(!query.contains("| stats count by host")); // Won't have full pipeline
    }

    #[test]
    fn test_cluster_topology_generator() {
        let generator = ClusterTopologyGenerator::new()
            .with_peer_count(5)
            .with_site_count(2);

        let topology = generator.generate();
        let peers = topology["peers"].as_array().unwrap();
        assert_eq!(peers.len(), 5);
    }

    #[test]
    fn test_log_entry_generator() {
        let generator = LogEntryGenerator::new().with_component("Metrics");
        let entry = generator.generate_one();
        assert_eq!(entry["component"], "Metrics");
    }

    #[test]
    fn test_user_generator() {
        let generator = UserGenerator::new();
        let users = generator.generate_many(5);
        assert_eq!(users["entry"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn test_app_generator() {
        let generator = AppGenerator::new();
        let apps = generator.generate_many(3);
        assert_eq!(apps["entry"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_index_generator() {
        let generator = IndexGenerator::new();
        let indexes = generator.generate_many(4);
        assert_eq!(indexes["entry"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_bulk_generator() {
        let generator = BulkGenerator::new();
        let dataset = generator.generate_dataset(2, 2, 2, 10, 5);
        assert!(dataset.contains_key("users"));
        assert!(dataset.contains_key("apps"));
        assert!(dataset.contains_key("indexes"));
    }

    #[test]
    fn test_null_handling() {
        let mut rng = rand::thread_rng();

        // None should never return true
        assert!(!NullHandling::None.should_be_null(&mut rng));

        // Dense should return true more often than Sparse
        let dense_count: usize = (0..1000)
            .filter(|_| NullHandling::Dense.should_be_null(&mut rng))
            .count();
        let sparse_count: usize = (0..1000)
            .filter(|_| NullHandling::Sparse.should_be_null(&mut rng))
            .count();
        assert!(dense_count > sparse_count);
    }

    #[test]
    fn test_search_results_with_nulls() {
        let generator = SearchResultsGenerator::new()
            .with_row_count(100)
            .with_column_count(10)
            .with_null_handling(NullHandling::Moderate);

        let results = generator.generate();
        let rows = results["results"].as_array().unwrap();

        // With moderate null handling, some rows should have fewer fields
        let field_counts: Vec<usize> = rows.iter().map(|r| r.as_object().unwrap().len()).collect();

        // Should have variation in field counts due to nulls
        let min_fields = *field_counts.iter().min().unwrap();
        let max_fields = *field_counts.iter().max().unwrap();
        assert!(min_fields < max_fields || min_fields <= 6); // Allow for some rows to have minimal fields
    }
}
