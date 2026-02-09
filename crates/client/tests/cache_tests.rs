//! Integration tests for client-side response caching.

use std::time::Duration;

use splunk_client::client::cache::{CacheConfig, CachePolicy, ResponseCache};

#[test]
fn test_cache_basic_operations() {
    // Create a cache with default settings
    let cache = ResponseCache::new();

    // Initially empty
    assert_eq!(cache.stats().entry_count, 0);
    assert!(cache.is_enabled());
}

#[test]
fn test_cache_policy_matching() {
    let config = CacheConfig::default();

    // Index endpoints should be cached
    assert!(
        matches!(
            config.policy_for("/services/data/indexes"),
            CachePolicy::CacheWithTtl(_)
        ),
        "Index endpoints should have a cache policy"
    );

    // Job endpoints should be cached
    assert!(
        matches!(
            config.policy_for("/services/search/jobs"),
            CachePolicy::CacheWithTtl(_)
        ),
        "Job endpoints should have a cache policy"
    );

    // Health endpoints should be cached
    assert!(
        matches!(
            config.policy_for("/services/server/health"),
            CachePolicy::CacheWithTtl(_)
        ),
        "Health endpoints should have a cache policy"
    );

    // Server info should be cached
    assert!(
        matches!(
            config.policy_for("/services/server/info"),
            CachePolicy::CacheWithTtl(_)
        ),
        "Server info endpoints should have a cache policy"
    );
}

#[test]
fn test_disabled_cache() {
    let cache = ResponseCache::disabled();
    assert!(!cache.is_enabled());

    // Stats should still work
    let stats = cache.stats();
    assert!(!stats.enabled);
}

#[test]
fn test_cache_enable_disable() {
    let mut cache = ResponseCache::new();
    assert!(cache.is_enabled());

    cache.disable();
    assert!(!cache.is_enabled());

    cache.enable();
    assert!(cache.is_enabled());
}

#[tokio::test]
async fn test_cache_insert_and_get() {
    use splunk_client::client::cache::{CacheEntry, CacheKey};

    let cache = ResponseCache::new();
    let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
    let entry = CacheEntry::new(
        b"test body".to_vec(),
        200,
        vec![("content-type".to_string(), "application/json".to_string())],
        Duration::from_secs(60),
    );

    // Initially empty
    assert!(cache.get(&key).await.is_none());

    // Insert
    cache.insert(key.clone(), entry.clone()).await;

    // Now should exist
    let retrieved = cache.get(&key).await;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.body, b"test body");
    assert_eq!(retrieved.status, 200);
}

#[tokio::test]
async fn test_cache_invalidate() {
    use splunk_client::client::cache::{CacheEntry, CacheKey};

    let cache = ResponseCache::new();
    let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
    let entry = CacheEntry::new(b"test".to_vec(), 200, vec![], Duration::from_secs(60));

    cache.insert(key.clone(), entry).await;
    assert!(cache.get(&key).await.is_some());

    cache.invalidate(&key).await;
    assert!(cache.get(&key).await.is_none());
}

#[tokio::test]
async fn test_cache_invalidate_all() {
    use splunk_client::client::cache::{CacheEntry, CacheKey};

    let cache = ResponseCache::new();

    // Insert multiple entries
    let key1 = CacheKey::new("https://example.com/api1".to_string(), vec![]);
    let key2 = CacheKey::new("https://example.com/api2".to_string(), vec![]);
    let entry = CacheEntry::new(b"test".to_vec(), 200, vec![], Duration::from_secs(60));

    cache.insert(key1.clone(), entry.clone()).await;
    cache.insert(key2.clone(), entry).await;

    assert!(cache.get(&key1).await.is_some());
    assert!(cache.get(&key2).await.is_some());

    // Invalidate all
    cache.invalidate_all().await;

    assert!(cache.get(&key1).await.is_none());
    assert!(cache.get(&key2).await.is_none());
}

#[tokio::test]
async fn test_cache_ttl_expiration() {
    use splunk_client::client::cache::{CacheEntry, CacheKey};

    let cache = ResponseCache::new();
    let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
    // Very short TTL for testing
    let entry = CacheEntry::new(b"test".to_vec(), 200, vec![], Duration::from_millis(10));

    cache.insert(key.clone(), entry).await;
    assert!(cache.get(&key).await.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Should be expired now
    assert!(cache.get(&key).await.is_none());
}

#[test]
fn test_cache_config_custom_policies() {
    let mut config = CacheConfig::default();

    // Set a custom policy
    config.set_policy("/custom/path", CachePolicy::NoCache);

    // Should return NoCache for custom path
    assert_eq!(
        config.policy_for("/custom/path/subpath"),
        CachePolicy::NoCache
    );

    // Other paths should still use default
    assert!(matches!(
        config.policy_for("/other/path"),
        CachePolicy::CacheWithTtl(_)
    ));
}

#[test]
fn test_cache_config_default_ttl() {
    let mut config = CacheConfig::default();

    // Change default TTL
    config.set_default_ttl(Duration::from_secs(300));

    // Unknown endpoints should use new default
    match config.policy_for("/unknown/path") {
        CachePolicy::CacheWithTtl(duration) => {
            assert_eq!(duration, Duration::from_secs(300));
        }
        _ => panic!("Expected CacheWithTtl policy"),
    }
}

#[test]
fn test_should_cache_request_get() {
    use reqwest::Method;

    let cache = ResponseCache::new();

    let policy = cache.should_cache_request(&Method::GET, "/services/data/indexes");
    assert!(
        matches!(policy, CachePolicy::CacheWithTtl(_)),
        "GET requests to index endpoints should be cacheable"
    );
}

#[test]
fn test_should_cache_request_post() {
    use reqwest::Method;

    let cache = ResponseCache::new();

    let policy = cache.should_cache_request(&Method::POST, "/services/data/indexes");
    assert_eq!(
        policy,
        CachePolicy::NoCache,
        "POST requests should not be cached"
    );
}

#[test]
fn test_should_cache_request_when_disabled() {
    use reqwest::Method;

    let mut cache = ResponseCache::new();
    cache.disable();

    let policy = cache.should_cache_request(&Method::GET, "/services/data/indexes");
    assert_eq!(
        policy,
        CachePolicy::NoCache,
        "Requests should not be cached when cache is disabled"
    );
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
fn test_parse_cache_control_case_insensitive() {
    // Header name is case-insensitive, but directive is case-sensitive (should be lowercase)
    let headers = vec![("Cache-Control".to_string(), "max-age=600".to_string())];

    let duration = ResponseCache::parse_cache_control(&headers);
    assert_eq!(duration, Some(Duration::from_secs(600)));
}

#[test]
fn test_cache_with_capacity() {
    let cache = ResponseCache::with_capacity(50);

    // Should be enabled with custom capacity
    assert!(cache.is_enabled());
    assert_eq!(cache.stats().entry_count, 0);
}

#[test]
fn test_cache_key_sorts_query_params() {
    use splunk_client::client::cache::CacheKey;

    // Same params in different order should produce equal keys
    let key1 = CacheKey::new(
        "https://example.com/api".to_string(),
        vec![
            ("z".to_string(), "last".to_string()),
            ("a".to_string(), "first".to_string()),
        ],
    );
    let key2 = CacheKey::new(
        "https://example.com/api".to_string(),
        vec![
            ("a".to_string(), "first".to_string()),
            ("z".to_string(), "last".to_string()),
        ],
    );

    assert_eq!(
        key1, key2,
        "Cache keys with same params in different order should be equal"
    );
}

#[test]
fn test_cache_key_different_urls() {
    use splunk_client::client::cache::CacheKey;

    let key1 = CacheKey::new("https://example.com/api1".to_string(), vec![]);
    let key2 = CacheKey::new("https://example.com/api2".to_string(), vec![]);

    assert_ne!(key1, key2, "Different URLs should produce different keys");
}

#[tokio::test]
async fn test_cache_entry_headers_preserved() {
    use splunk_client::client::cache::{CacheEntry, CacheKey};

    let cache = ResponseCache::new();
    let key = CacheKey::new("https://example.com/api".to_string(), vec![]);
    let headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("etag".to_string(), "\"abc123\"".to_string()),
    ];
    let entry = CacheEntry::new(
        b"test".to_vec(),
        200,
        headers.clone(),
        Duration::from_secs(60),
    );

    cache.insert(key.clone(), entry).await;

    let retrieved = cache.get(&key).await.unwrap();
    assert_eq!(retrieved.headers, headers);
}

#[test]
fn test_cache_stats_display() {
    use splunk_client::client::cache::CacheStats;

    let stats = CacheStats {
        entry_count: 42,
        enabled: true,
    };
    let display = format!("{}", stats);
    assert!(display.contains("42"), "Display should show entry count");
    assert!(
        display.contains("true"),
        "Display should show enabled status"
    );
}

#[test]
fn test_cache_entry_expiration_check() {
    use splunk_client::client::cache::CacheEntry;

    // Entry with short TTL
    let entry = CacheEntry::new(b"test".to_vec(), 200, vec![], Duration::from_millis(1));

    // Should not be expired immediately
    assert!(!entry.is_expired());

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(10));

    // Should be expired now
    assert!(entry.is_expired());
}

#[test]
fn test_cache_entry_not_expired() {
    use splunk_client::client::cache::CacheEntry;

    // Entry with long TTL
    let entry = CacheEntry::new(b"test".to_vec(), 200, vec![], Duration::from_secs(3600));

    // Should not be expired
    assert!(!entry.is_expired());
}
