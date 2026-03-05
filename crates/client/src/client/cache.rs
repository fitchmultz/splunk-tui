//! Client-side response caching for API requests.
//!
//! Purpose: Provide in-memory response caching for idempotent client reads.
//! Responsibilities: Cache GET responses, apply endpoint-specific TTL policy, and expose cache metrics.
//! Non-scope: Persistent storage, distributed cache coordination, or proactive pre-warming.
//! Invariants/Assumptions: Only safe-to-cache reads are stored and cache invalidation follows mutating operations.
//!
//! This module provides an LRU cache with TTL support for HTTP responses,
//! reducing server load and improving response times for repeated queries.
//!
//! # Features
//! - Per-endpoint TTL configuration based on data volatility
//! - Automatic cache invalidation on mutating operations (POST/PUT/DELETE)
//! - Cache-Control header respect for server-directed caching
//! - Cache statistics for monitoring hit/miss rates
//!
//! # What this module does NOT handle:
//! - Persistent disk caching (in-memory only)
//! - Cross-process cache sharing
//! - Cache warming or pre-population
//!
//! # Invariants
//! - Only GET requests are cached
//! - Mutating operations invalidate related cache entries
//! - TTL is enforced per-entry, not globally

use std::time::{Duration, Instant};

use moka::future::Cache as MokaCache;
use moka::policy::EvictionPolicy;
use reqwest::Method;
use tracing::{debug, trace};

use crate::metrics::{METRIC_CACHE_HITS, METRIC_CACHE_MISSES, MetricsCollector};

/// Default cache size (number of entries).
pub const DEFAULT_CACHE_SIZE: u64 = 100;

/// Default TTL for index-related endpoints (60 seconds).
pub const DEFAULT_INDEX_TTL_SECONDS: u64 = 60;

/// Default TTL for job-related endpoints (10 seconds).
pub const DEFAULT_JOB_TTL_SECONDS: u64 = 10;

/// Default TTL for health endpoints (30 seconds).
pub const DEFAULT_HEALTH_TTL_SECONDS: u64 = 30;

/// Default TTL for server info endpoints (300 seconds - 5 minutes).
pub const DEFAULT_SERVER_INFO_TTL_SECONDS: u64 = 300;

/// Default TTL for general list endpoints (120 seconds).
pub const DEFAULT_LIST_TTL_SECONDS: u64 = 120;

/// Metric name for cache eviction counter.
pub const METRIC_CACHE_EVICTIONS: &str = "splunk_api_cache_evictions_total";

fn env_u64_or_default(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// A cached HTTP response entry.
#[derive(Clone, Debug)]
pub struct CacheEntry {
    /// The response body as bytes.
    pub body: Vec<u8>,
    /// HTTP status code.
    pub status: u16,
    /// Response headers (filtered to cacheable ones).
    pub headers: Vec<(String, String)>,
    /// When this entry was cached.
    pub cached_at: Instant,
    /// Time-to-live for this entry.
    pub ttl: Duration,
}

impl CacheEntry {
    /// Check if this cache entry has expired relative to a given reference time.
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now.duration_since(self.cached_at) > self.ttl
    }

    /// Check if this cache entry has expired (uses wall-clock time).
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    /// Create a cache entry from an HTTP response.
    ///
    /// # Arguments
    /// * `body` - The response body bytes
    /// * `status` - HTTP status code
    /// * `headers` - Response headers
    /// * `ttl` - Time-to-live duration
    pub fn new(body: Vec<u8>, status: u16, headers: Vec<(String, String)>, ttl: Duration) -> Self {
        Self {
            body,
            status,
            headers,
            cached_at: Instant::now(),
            ttl,
        }
    }
}

/// Cache key for identifying cached requests.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CacheKey {
    /// The full request URL.
    pub url: String,
    /// Query parameters (sorted for consistency).
    pub query_params: Vec<(String, String)>,
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(url: String, mut query_params: Vec<(String, String)>) -> Self {
        // Sort query params for consistent keys regardless of order
        query_params.sort_by(|a, b| a.0.cmp(&b.0));
        Self { url, query_params }
    }
}

/// Cache policy for an endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CachePolicy {
    /// Never cache this endpoint.
    NoCache,
    /// Cache with a specific TTL.
    CacheWithTtl(Duration),
    /// Cache with TTL determined by Cache-Control header.
    RespectCacheControl,
}

/// Configuration for cache policies per endpoint.
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Policies mapped by endpoint path prefix.
    policies: std::collections::HashMap<String, CachePolicy>,
    /// Default TTL when no specific policy matches.
    default_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let mut policies = std::collections::HashMap::new();
        let index_ttl_seconds =
            env_u64_or_default("SPLUNK_CACHE_TTL_INDEX_SECONDS", DEFAULT_INDEX_TTL_SECONDS);
        let job_ttl_seconds =
            env_u64_or_default("SPLUNK_CACHE_TTL_JOB_SECONDS", DEFAULT_JOB_TTL_SECONDS);
        let health_ttl_seconds = env_u64_or_default(
            "SPLUNK_CACHE_TTL_HEALTH_SECONDS",
            DEFAULT_HEALTH_TTL_SECONDS,
        );
        let server_info_ttl_seconds = env_u64_or_default(
            "SPLUNK_CACHE_TTL_SERVER_INFO_SECONDS",
            DEFAULT_SERVER_INFO_TTL_SECONDS,
        );
        let list_ttl_seconds =
            env_u64_or_default("SPLUNK_CACHE_TTL_LIST_SECONDS", DEFAULT_LIST_TTL_SECONDS);

        // Index endpoints - cache for 60 seconds
        policies.insert(
            "/services/data/indexes".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(index_ttl_seconds)),
        );

        // Job endpoints - cache for 10 seconds (highly volatile)
        policies.insert(
            "/services/search/jobs".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(job_ttl_seconds)),
        );

        // Health endpoints - cache for 30 seconds
        policies.insert(
            "/services/server/health".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(health_ttl_seconds)),
        );

        // Server info - cache for 5 minutes (rarely changes)
        policies.insert(
            "/services/server/info".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(server_info_ttl_seconds)),
        );

        // Forwarders - cache for 2 minutes
        policies.insert(
            "/services/deployment/server/clients".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(list_ttl_seconds)),
        );

        // Search peers - cache for 2 minutes
        policies.insert(
            "/services/search/distributed/peers".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(list_ttl_seconds)),
        );

        // Cluster peers - cache for 2 minutes
        policies.insert(
            "/services/cluster/master/peers".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(list_ttl_seconds)),
        );

        // License info - cache for 5 minutes
        policies.insert(
            "/services/licenser/usage".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(server_info_ttl_seconds)),
        );

        // KVStore status - cache for 30 seconds
        policies.insert(
            "/services/kvstore/status".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(health_ttl_seconds)),
        );

        // Apps - cache for 5 minutes
        policies.insert(
            "/services/apps/local".to_string(),
            CachePolicy::CacheWithTtl(Duration::from_secs(server_info_ttl_seconds)),
        );

        Self {
            policies,
            default_ttl: Duration::from_secs(list_ttl_seconds),
        }
    }
}

impl CacheConfig {
    /// Get the cache policy for a given endpoint path.
    pub fn policy_for(&self, endpoint: &str) -> CachePolicy {
        // Find the longest matching prefix
        let mut best_match: Option<(&str, CachePolicy)> = None;

        for (prefix, policy) in &self.policies {
            if endpoint.starts_with(prefix)
                && best_match.is_none_or(|(current_prefix, _)| prefix.len() > current_prefix.len())
            {
                best_match = Some((prefix, *policy));
            }
        }

        best_match.map_or(
            CachePolicy::CacheWithTtl(self.default_ttl),
            |(_, policy)| policy,
        )
    }

    /// Set a custom policy for an endpoint prefix.
    pub fn set_policy(&mut self, prefix: impl Into<String>, policy: CachePolicy) {
        self.policies.insert(prefix.into(), policy);
    }

    /// Set the default TTL.
    pub fn set_default_ttl(&mut self, ttl: Duration) {
        self.default_ttl = ttl;
    }
}

/// Client-side response cache.
#[derive(Clone, Debug)]
pub struct ResponseCache {
    /// The underlying Moka cache.
    inner: MokaCache<CacheKey, CacheEntry>,
    /// Cache configuration with per-endpoint policies.
    config: CacheConfig,
    /// Whether caching is enabled.
    enabled: bool,
    /// Optional metrics collector.
    metrics: Option<MetricsCollector>,
}

impl ResponseCache {
    /// Create a new response cache with default settings.
    pub fn new() -> Self {
        let capacity = env_u64_or_default("SPLUNK_CACHE_SIZE", DEFAULT_CACHE_SIZE);
        Self::with_capacity(capacity)
    }

    /// Create a new response cache with a specific capacity.
    pub fn with_capacity(capacity: u64) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(capacity)
            .eviction_policy(EvictionPolicy::lru())
            .build();

        Self {
            inner: cache,
            config: CacheConfig::default(),
            enabled: true,
            metrics: None,
        }
    }

    /// Create a disabled cache (no caching).
    pub fn disabled() -> Self {
        Self {
            inner: MokaCache::builder().max_capacity(1).build(),
            config: CacheConfig::default(),
            enabled: false,
            metrics: None,
        }
    }

    /// Set the metrics collector.
    pub fn with_metrics(mut self, metrics: MetricsCollector) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Set the cache configuration.
    pub fn with_config(mut self, config: CacheConfig) -> Self {
        self.config = config;
        self
    }

    /// Disable the cache.
    pub fn disable(&mut self) {
        self.enabled = false;
        debug!("Response cache disabled");
    }

    /// Enable the cache.
    pub fn enable(&mut self) {
        self.enabled = true;
        debug!("Response cache enabled");
    }

    /// Check if caching is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get a mutable reference to the cache configuration.
    pub fn config_mut(&mut self) -> &mut CacheConfig {
        &mut self.config
    }

    /// Get an entry from the cache.
    pub async fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        self.get_at(key, Instant::now()).await
    }

    /// Get an entry from the cache, checking expiration relative to a given time.
    pub async fn get_at(&self, key: &CacheKey, now: Instant) -> Option<CacheEntry> {
        if !self.enabled {
            return None;
        }

        match self.inner.get(key).await {
            Some(entry) => {
                if entry.is_expired_at(now) {
                    trace!("Cache entry expired for key: {}", key.url);
                    self.inner.invalidate(key).await;
                    self.record_miss();
                    None
                } else {
                    trace!("Cache hit for key: {}", key.url);
                    self.record_hit();
                    Some(entry)
                }
            }
            None => {
                trace!("Cache miss for key: {}", key.url);
                self.record_miss();
                None
            }
        }
    }

    /// Store an entry in the cache.
    pub async fn insert(&self, key: CacheKey, entry: CacheEntry) {
        if !self.enabled {
            return;
        }

        trace!("Caching entry for key: {}", key.url);
        self.inner.insert(key, entry).await;
    }

    /// Invalidate a specific cache entry.
    pub async fn invalidate(&self, key: &CacheKey) {
        self.inner.invalidate(key).await;
        self.record_eviction_by(1);
        trace!("Invalidated cache entry for key: {}", key.url);
    }

    /// Invalidate all entries matching an endpoint prefix.
    pub async fn invalidate_prefix(&self, prefix: &str) {
        // Note: Moka doesn't support prefix invalidation directly,
        // so we iterate and check. For high-volume caches, consider
        // using a different invalidation strategy.
        let prefix_owned = prefix.to_string();
        self.inner
            .invalidate_entries_if(move |key, _| key.url.starts_with(&prefix_owned))
            .ok();
        // Moka doesn't expose affected-entry count for predicate invalidation.
        // Record this as one invalidation event for observability.
        self.record_eviction_by(1);
        debug!("Invalidated cache entries with prefix: {}", prefix);
    }

    /// Invalidate all cache entries.
    pub async fn invalidate_all(&self) {
        let evicted = self.inner.entry_count();
        self.inner.invalidate_all();
        self.record_eviction_by(evicted);
        debug!("Invalidated all cache entries");
    }

    /// Determine if a request should be cached based on method and endpoint.
    pub fn should_cache_request(&self, method: &Method, endpoint: &str) -> CachePolicy {
        if !self.enabled || *method != Method::GET {
            return CachePolicy::NoCache;
        }

        self.config.policy_for(endpoint)
    }

    /// Parse Cache-Control header to extract max-age.
    pub fn parse_cache_control(headers: &[(String, String)]) -> Option<Duration> {
        for (name, value) in headers {
            if name.eq_ignore_ascii_case("cache-control") {
                // Parse max-age directive
                for directive in value.split(',') {
                    let directive = directive.trim();
                    if let Some(stripped) = directive.strip_prefix("max-age=") {
                        if let Ok(seconds) = stripped.parse::<u64>() {
                            return Some(Duration::from_secs(seconds));
                        }
                    }
                }
            }
        }
        None
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.inner.entry_count(),
            enabled: self.enabled,
        }
    }

    /// Record a cache hit.
    fn record_hit(&self) {
        if self.metrics.is_some() {
            metrics::counter!(METRIC_CACHE_HITS).increment(1);
        }
    }

    /// Record a cache miss.
    fn record_miss(&self) {
        if self.metrics.is_some() {
            metrics::counter!(METRIC_CACHE_MISSES).increment(1);
        }
    }

    /// Record cache eviction events.
    fn record_eviction_by(&self, count: u64) {
        if self.metrics.is_some() && count > 0 {
            metrics::counter!(METRIC_CACHE_EVICTIONS).increment(count);
        }
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    /// Number of entries in the cache.
    pub entry_count: u64,
    /// Whether caching is enabled.
    pub enabled: bool,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache {{ entries: {}, enabled: {} }}",
            self.entry_count, self.enabled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default_policies() {
        let config = CacheConfig::default();

        // Index endpoints should have 60s TTL
        match config.policy_for("/services/data/indexes") {
            CachePolicy::CacheWithTtl(duration) => {
                assert_eq!(duration, Duration::from_secs(60));
            }
            _ => panic!("Expected CacheWithTtl policy for indexes"),
        }

        // Job endpoints should have 10s TTL
        match config.policy_for("/services/search/jobs") {
            CachePolicy::CacheWithTtl(duration) => {
                assert_eq!(duration, Duration::from_secs(10));
            }
            _ => panic!("Expected CacheWithTtl policy for jobs"),
        }

        // Unknown endpoints should use default TTL
        match config.policy_for("/unknown/endpoint") {
            CachePolicy::CacheWithTtl(_) => {} // Any duration is fine
            _ => panic!("Expected CacheWithTtl policy for unknown endpoints"),
        }
    }

    #[test]
    fn test_cache_config_set_policy() {
        let mut config = CacheConfig::default();
        config.set_policy("/custom", CachePolicy::NoCache);

        assert_eq!(config.policy_for("/custom/path"), CachePolicy::NoCache);
    }

    #[test]
    fn test_cache_key_sorts_params() {
        let key1 = CacheKey::new(
            "https://example.com/api".to_string(),
            vec![
                ("b".to_string(), "2".to_string()),
                ("a".to_string(), "1".to_string()),
            ],
        );
        let key2 = CacheKey::new(
            "https://example.com/api".to_string(),
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
        );

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new(vec![1, 2, 3], 200, vec![], Duration::from_millis(1));

        assert!(!entry.is_expired());
        std::thread::sleep(Duration::from_millis(2));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_parse_cache_control() {
        let headers = vec![(
            "Cache-Control".to_string(),
            "max-age=300, must-revalidate".to_string(),
        )];

        let duration = ResponseCache::parse_cache_control(&headers);
        assert_eq!(duration, Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_parse_cache_control_no_max_age() {
        let headers = vec![(
            "Cache-Control".to_string(),
            "no-cache, must-revalidate".to_string(),
        )];

        let duration = ResponseCache::parse_cache_control(&headers);
        assert_eq!(duration, None);
    }

    #[test]
    fn test_should_cache_request_get() {
        let cache = ResponseCache::new();

        let policy = cache.should_cache_request(&Method::GET, "/services/data/indexes");
        assert!(matches!(policy, CachePolicy::CacheWithTtl(_)));
    }

    #[test]
    fn test_should_cache_request_post() {
        let cache = ResponseCache::new();

        let policy = cache.should_cache_request(&Method::POST, "/services/data/indexes");
        assert_eq!(policy, CachePolicy::NoCache);
    }

    #[test]
    fn test_should_cache_request_disabled() {
        let mut cache = ResponseCache::new();
        cache.disable();

        let policy = cache.should_cache_request(&Method::GET, "/services/data/indexes");
        assert_eq!(policy, CachePolicy::NoCache);
    }

    #[test]
    fn test_disabled_cache() {
        let cache = ResponseCache::disabled();
        assert!(!cache.is_enabled());
    }

    #[tokio::test]
    async fn test_cache_get_insert() {
        let cache = ResponseCache::new();
        let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
        let entry = CacheEntry::new(vec![1, 2, 3], 200, vec![], Duration::from_secs(60));

        // Initially empty
        assert!(cache.get(&key).await.is_none());

        // Insert
        cache.insert(key.clone(), entry).await;

        // Now should exist
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().body, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = ResponseCache::new();
        let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
        let entry = CacheEntry::new(vec![1, 2, 3], 200, vec![], Duration::from_secs(60));

        cache.insert(key.clone(), entry).await;
        assert!(cache.get(&key).await.is_some());

        cache.invalidate(&key).await;
        assert!(cache.get(&key).await.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ResponseCache::new();
        let stats = cache.stats();
        assert!(stats.enabled);
        assert_eq!(stats.entry_count, 0);
    }

    #[test]
    fn test_cache_stats_display() {
        let stats = CacheStats {
            entry_count: 42,
            enabled: true,
        };
        let display = format!("{}", stats);
        assert!(display.contains("42"));
        assert!(display.contains("true"));
    }
}
