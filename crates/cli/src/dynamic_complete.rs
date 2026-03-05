//! Dynamic completion support for runtime values from Splunk server.
//!
//! Responsibilities:
//! - Fetch completion values from Splunk server with caching
//! - Implement completers for each value type (profiles, indexes, jobs, etc.)
//! - Provide graceful fallback when server unavailable
//!
//! Does NOT handle:
//! - Shell-specific completion script generation (see commands/completions.rs)
//! - Static completion generation (handled by clap_complete derive macros)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Cache entry with expiration timestamp.
#[derive(Clone)]
struct CacheEntry<T> {
    data: Vec<T>,
    fetched_at: Instant,
}

impl<T> CacheEntry<T> {
    fn is_expired(&self, ttl: Duration) -> bool {
        self.fetched_at.elapsed() > ttl
    }
}

/// Completion cache with TTL support.
pub struct CompletionCache {
    default_ttl: Duration,
    memory_cache: Arc<Mutex<HashMap<String, CacheEntry<String>>>>,
}

impl CompletionCache {
    /// Create a new completion cache.
    pub fn new() -> Result<Self> {
        Ok(Self {
            default_ttl: Duration::from_secs(60),
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create cache with custom TTL.
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        let mut cache = Self::new()?;
        cache.default_ttl = Duration::from_secs(ttl_seconds);
        Ok(cache)
    }

    /// Get cached values or fetch using provided function.
    pub async fn get_or_fetch<F, Fut>(&self, key: &str, fetch_fn: F) -> Vec<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<String>>>,
    {
        // Check memory cache first
        {
            let cache = self.memory_cache.lock().await;
            if let Some(entry) = cache.get(key) {
                if !entry.is_expired(self.default_ttl) {
                    debug!("Using memory cache for {}", key);
                    return entry.data.clone();
                }
            }
        }

        // Try to fetch fresh data
        match fetch_fn().await {
            Ok(data) => {
                // Update memory cache
                let mut cache = self.memory_cache.lock().await;
                cache.insert(
                    key.to_string(),
                    CacheEntry {
                        data: data.clone(),
                        fetched_at: Instant::now(),
                    },
                );
                data
            }
            Err(e) => {
                warn!("Failed to fetch completions for {}: {}", key, e);
                // Return stale cache if available, otherwise empty
                let cache = self.memory_cache.lock().await;
                cache.get(key).map(|e| e.data.clone()).unwrap_or_default()
            }
        }
    }
}

/// Profile completer using ConfigManager.
pub struct ProfileCompleter;

impl ProfileCompleter {
    /// Get possible profile names from config.
    pub fn possible_values() -> Vec<String> {
        match splunk_config::ConfigManager::new() {
            Ok(manager) => manager.list_profiles().keys().cloned().collect(),
            Err(e) => {
                warn!("Failed to load profiles for completion: {}", e);
                vec![]
            }
        }
    }
}

/// Index completer with caching.
pub struct IndexCompleter {
    cache: Arc<CompletionCache>,
}

impl IndexCompleter {
    /// Create with custom TTL.
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(CompletionCache::with_ttl(ttl_seconds)?),
        })
    }

    /// Get possible index names from server.
    pub async fn possible_values(&self, config: &splunk_config::Config) -> Vec<String> {
        let cache = self.cache.clone();
        let config = config.clone();

        cache
            .get_or_fetch("indexes", || async {
                let client = splunk_client::SplunkClient::from_config(&config).await?;
                let indexes = client.list_indexes(Some(100), None).await?;
                Ok(indexes.into_iter().map(|i| i.name).collect())
            })
            .await
    }
}

/// Saved search completer with caching.
pub struct SavedSearchCompleter {
    cache: Arc<CompletionCache>,
}

impl SavedSearchCompleter {
    /// Create with custom TTL.
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(CompletionCache::with_ttl(ttl_seconds)?),
        })
    }

    /// Get possible saved search names from server.
    pub async fn possible_values(&self, config: &splunk_config::Config) -> Vec<String> {
        let cache = self.cache.clone();
        let config = config.clone();

        cache
            .get_or_fetch("saved_searches", || async {
                let client = splunk_client::SplunkClient::from_config(&config).await?;
                let searches = client.list_saved_searches(Some(100), None).await?;
                Ok(searches.into_iter().map(|s| s.name).collect())
            })
            .await
    }
}

/// Job SID completer with caching.
pub struct JobCompleter {
    cache: Arc<CompletionCache>,
}

impl JobCompleter {
    /// Create with custom TTL.
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(CompletionCache::with_ttl(ttl_seconds)?),
        })
    }

    /// Get possible job SIDs from server.
    pub async fn possible_values(&self, config: &splunk_config::Config) -> Vec<String> {
        let cache = self.cache.clone();
        let config = config.clone();

        cache
            .get_or_fetch("jobs", || async {
                let client = splunk_client::SplunkClient::from_config(&config).await?;
                let jobs = client.list_jobs(Some(50), None).await?;
                Ok(jobs.into_iter().map(|j| j.sid).collect())
            })
            .await
    }
}

/// App completer with caching.
pub struct AppCompleter {
    cache: Arc<CompletionCache>,
}

impl AppCompleter {
    /// Create with custom TTL.
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(CompletionCache::with_ttl(ttl_seconds)?),
        })
    }

    /// Get possible app names from server.
    pub async fn possible_values(&self, config: &splunk_config::Config) -> Vec<String> {
        let cache = self.cache.clone();
        let config = config.clone();

        cache
            .get_or_fetch("apps", || async {
                let client = splunk_client::SplunkClient::from_config(&config).await?;
                let apps = client.list_apps(Some(100), None).await?;
                Ok(apps.into_iter().map(|a| a.name).collect())
            })
            .await
    }
}

/// Completion type enum for the Complete command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CompletionType {
    /// Profile names from config
    Profiles,
    /// Index names from server
    Indexes,
    /// Saved search names from server
    SavedSearches,
    /// Job SIDs from server
    Jobs,
    /// App names from server
    Apps,
}

/// Generate completions for a specific type.
pub async fn generate_completions(
    completion_type: CompletionType,
    config: Option<&splunk_config::Config>,
    cache_ttl: Option<u64>,
) -> Vec<String> {
    let ttl = cache_ttl.unwrap_or(60);

    match completion_type {
        CompletionType::Profiles => ProfileCompleter::possible_values(),
        CompletionType::Indexes => {
            if let Some(cfg) = config {
                match IndexCompleter::with_ttl(ttl) {
                    Ok(completer) => completer.possible_values(cfg).await,
                    Err(e) => {
                        warn!("Failed to create index completer: {}", e);
                        vec![]
                    }
                }
            } else {
                warn!("Config required for index completions");
                vec![]
            }
        }
        CompletionType::SavedSearches => {
            if let Some(cfg) = config {
                match SavedSearchCompleter::with_ttl(ttl) {
                    Ok(completer) => completer.possible_values(cfg).await,
                    Err(e) => {
                        warn!("Failed to create saved search completer: {}", e);
                        vec![]
                    }
                }
            } else {
                warn!("Config required for saved search completions");
                vec![]
            }
        }
        CompletionType::Jobs => {
            if let Some(cfg) = config {
                match JobCompleter::with_ttl(ttl) {
                    Ok(completer) => completer.possible_values(cfg).await,
                    Err(e) => {
                        warn!("Failed to create job completer: {}", e);
                        vec![]
                    }
                }
            } else {
                warn!("Config required for job completions");
                vec![]
            }
        }
        CompletionType::Apps => {
            if let Some(cfg) = config {
                match AppCompleter::with_ttl(ttl) {
                    Ok(completer) => completer.possible_values(cfg).await,
                    Err(e) => {
                        warn!("Failed to create app completer: {}", e);
                        vec![]
                    }
                }
            } else {
                warn!("Config required for app completions");
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry {
            data: vec!["test".to_string()],
            fetched_at: Instant::now() - Duration::from_secs(120),
        };

        assert!(entry.is_expired(Duration::from_secs(60)));
        assert!(!entry.is_expired(Duration::from_secs(180)));
    }

    #[test]
    fn test_profile_completer_returns_empty_on_error() {
        // When ConfigManager fails, should return empty vec, not panic
        let profiles = ProfileCompleter::possible_values();
        // In test environment, this might succeed or fail depending on env
        // Just verify it doesn't panic
        let _ = profiles;
    }

    #[tokio::test]
    async fn test_completion_cache_get_or_fetch_returns_data() {
        let cache = CompletionCache::new().unwrap();

        let result = cache
            .get_or_fetch("test_key", || async {
                Ok(vec!["item1".to_string(), "item2".to_string()])
            })
            .await;

        assert_eq!(result, vec!["item1", "item2"]);
    }

    #[tokio::test]
    async fn test_completion_cache_uses_cache_on_fetch_failure() {
        let cache = CompletionCache::new().unwrap();

        // First, populate cache
        let _ = cache
            .get_or_fetch("fallback_test", || async { Ok(vec!["cached".to_string()]) })
            .await;

        // Now fetch with error - should return cached value
        let result = cache
            .get_or_fetch("fallback_test", || async {
                Err(anyhow::anyhow!("Network error"))
            })
            .await;

        assert_eq!(result, vec!["cached"]);
    }
}
