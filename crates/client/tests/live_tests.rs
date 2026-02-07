//! Live server tests against a real Splunk instance.
//!
//! These tests require a reachable Splunk server configured via environment
//! variables or `.env.test` (workspace root).
//!
//! These tests are designed to be "best effort":
//! - If required `SPLUNK_*` variables are not set, the tests no-op (pass).
//! - If the configured server is unreachable, the tests no-op (pass).
//! - If the server is reachable but requests fail (auth, API errors), the tests fail.
//!
//! Run with: cargo test -p splunk-client --test live_tests -- --ignored

mod live;

// Include all live test modules
mod auth_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_login() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        // Login by calling any authenticated method
        // If this succeeds without error, login worked
        client
            .list_indexes(Some(1), Some(0))
            .await
            .expect("Login failed");
    }
}

mod indexes_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_list_indexes() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let indexes = client
            .list_indexes(Some(500), Some(0))
            .await
            .expect("Failed to list indexes");

        assert!(!indexes.is_empty(), "Should have at least one index");
        assert!(
            indexes.iter().any(|i| i.name == "main"),
            "Should have 'main' index"
        );
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_list_indexes_pagination() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // Test count limit
        let limited = client
            .list_indexes(Some(1), Some(0))
            .await
            .expect("Failed to list indexes");
        assert_eq!(limited.len(), 1, "count=1 should return exactly 1 index");

        // Test offset
        let first_page = client
            .list_indexes(Some(10), Some(0))
            .await
            .expect("Failed to list indexes");
        let second_page = client
            .list_indexes(Some(10), Some(1))
            .await
            .expect("Failed to list indexes");

        // Second page should not contain the first item from first page
        if !first_page.is_empty() && !second_page.is_empty() {
            assert_ne!(
                first_page[0].name, second_page[0].name,
                "offset should shift results"
            );
        }
    }
}

mod search_live_tests {
    use std::time::Duration;

    use super::live::*;
    use splunk_client::endpoints::search::CreateJobOptions;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_search_and_get_results() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // Create a search job
        let sid = client
            .create_search_job(
                r#"| makeresults | eval foo="bar" | table foo"#,
                &CreateJobOptions {
                    wait: Some(true),
                    exec_time: Some(60),
                    ..Default::default()
                },
            )
            .await
            .expect("Failed to create search job");

        // Even with `wait=true`, Splunk can briefly return an empty results page.
        // Poll until we see the expected row, or time out.
        let mut last_total = None;
        for _ in 0..20 {
            // get_search_results takes u64, not Option<u64>
            let results = client
                .get_search_results(&sid, 5, 0)
                .await
                .expect("Failed to get search results");
            last_total = results.total;

            if let Some(first) = results.results.first()
                && first.get("foo").and_then(|v| v.as_str()) == Some("bar")
            {
                return;
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        panic!(
            "Search results did not contain expected foo=bar row (last total={:?})",
            last_total
        );
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_create_status_and_delete_job() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        let sid = client
            .create_search_job(
                r#"| makeresults | eval foo="job" | table foo"#,
                &CreateJobOptions {
                    wait: Some(false),
                    exec_time: Some(60),
                    ..Default::default()
                },
            )
            .await
            .expect("Failed to create search job");

        let _status = client
            .get_job_status(&sid)
            .await
            .expect("Failed to get job status");

        client.delete_job(&sid).await.expect("Failed to delete job");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_list_jobs() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        // Just verify we can list jobs successfully
        let _jobs = client
            .list_jobs(Some(10), Some(0))
            .await
            .expect("Failed to list jobs");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_create_and_cancel_job() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // Create a search job without waiting
        let sid = client
            .create_search_job(
                r#"| makeresults | eval foo="cancel" | table foo"#,
                &CreateJobOptions {
                    wait: Some(false),
                    ..Default::default()
                },
            )
            .await
            .expect("Failed to create search job");

        // Cancel the job
        client.cancel_job(&sid).await.expect("Failed to cancel job");
    }
}

mod cluster_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_cluster_info() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // This may fail on standalone instances - just verify we can make the call
        let _result = client.get_cluster_info().await;
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_cluster_peers() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // This may return empty on standalone instances - just verify we can make the call
        let _peers = client.get_cluster_peers().await;
    }
}

mod server_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_server_info() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let info = client
            .get_server_info()
            .await
            .expect("Failed to get server info");

        assert!(
            !info.server_name.is_empty(),
            "server_name should not be empty"
        );
        assert!(!info.version.is_empty(), "version should not be empty");
        assert!(!info.build.is_empty(), "build should not be empty");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_health() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let health = client.get_health().await.expect("Failed to get health");

        assert!(!health.health.is_empty(), "health should not be empty");
    }
}

mod license_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_license_usage() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let usage = client
            .get_license_usage()
            .await
            .expect("Failed to get license usage");

        assert!(!usage.is_empty(), "license usage should not be empty");
        assert!(
            usage.iter().all(|u| u.quota > 0),
            "all license entries should have a quota"
        );
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_list_license_pools_and_stacks() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        let _pools = client
            .list_license_pools()
            .await
            .expect("Failed to list license pools");
        let _stacks = client
            .list_license_stacks()
            .await
            .expect("Failed to list license stacks");
    }
}

mod kvstore_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_kvstore_status() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let status = client
            .get_kvstore_status()
            .await
            .expect("Failed to get KVStore status");

        assert!(
            !status.current_member.host.is_empty(),
            "KVStore current member host should not be empty"
        );
        assert!(
            status.current_member.port > 0,
            "KVStore current member port should be > 0"
        );
    }
}

mod diagnostics_live_tests {
    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_check_log_parsing_health() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let parsing = client
            .check_log_parsing_health()
            .await
            .expect("Failed to check log parsing health");

        assert_eq!(
            parsing.total_errors,
            parsing.errors.len(),
            "total_errors should match the number of error entries"
        );
        assert!(
            !parsing.time_window.is_empty(),
            "time_window should not be empty"
        );
        assert_eq!(
            parsing.is_healthy,
            parsing.total_errors == 0,
            "is_healthy should reflect whether errors were found"
        );
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_internal_logs() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };
        let logs = client
            .get_internal_logs(20, Some("-15m"))
            .await
            .expect("Failed to get internal logs");

        assert!(
            logs.len() <= 20,
            "returned logs should not exceed requested count"
        );
        assert!(
            logs.iter()
                .all(|l| !l.time.is_empty() && !l.message.is_empty()),
            "log entries should have time and message"
        );
    }
}

mod apps_live_tests {
    use std::time::Duration;

    use super::live::*;

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_list_apps_and_users_and_saved_searches() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        let apps = client
            .list_apps(Some(10), Some(0))
            .await
            .expect("Failed to list apps");
        assert!(!apps.is_empty(), "apps list should not be empty");

        let users = client
            .list_users(Some(50), Some(0))
            .await
            .expect("Failed to list users");
        assert!(
            users.iter().any(|u| u.name == "admin"),
            "users should include an 'admin' user"
        );

        // Saved searches may be empty depending on instance configuration; this is a smoke test.
        let _saved_searches = client
            .list_saved_searches(None, None)
            .await
            .expect("Failed to list saved searches");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_app() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // Test getting the "search" app which always exists
        let app = client.get_app("search").await.expect("Failed to get app");
        assert_eq!(app.name, "search");
        assert!(!app.label.as_deref().unwrap_or("").is_empty());
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_enable_disable_app() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        // Find an app that's safe to toggle (not a core system app)
        // Get the list of apps and find one that's not critical
        let apps = client
            .list_apps(Some(50), Some(0))
            .await
            .expect("Failed to list apps");

        // Look for a non-critical app that's visible and configured
        // Avoid: search, splunk_instrumentation, splunk_assist, etc.
        let test_app = apps
            .iter()
            .find(|a| {
                !a.disabled
                    && a.is_visible.unwrap_or(false)
                    && !matches!(
                        a.name.as_str(),
                        "search"
                            | "splunk_instrumentation"
                            | "splunk_assist"
                            | "splunk_instance_monitoring"
                            | "splunk_metrics_workspace"
                            | "splunk_monitoring_console"
                            | "splunk_rapid_diagnosis"
                            | "launcher"
                            | "learned"
                            | "legacy"
                            | "sample_app"
                            | "splunk_archiver"
                            | "splunk_httpinput"
                            | "splunk_internal_metrics"
                            | "splunk_secure_gateway"
                            | "splunk_telemetry"
                            | "introspection_generator_addon"
                            | "journald_input"
                            | "python_upgrade_readiness_app"
                            | "splunk_essentials_9_4"
                            | "splunk_gdi"
                            | "splunk_ingest_actions"
                            | "splunk_react_ui"
                            | "splunk_wft"
                            | "user_prefs"
                    )
            })
            .map(|a| a.name.clone());

        let Some(app_name) = test_app else {
            eprintln!("Skipping enable/disable test: no suitable test app found");
            return;
        };

        // First disable, then re-enable to restore state
        client
            .disable_app(&app_name)
            .await
            .expect("Failed to disable app");
        client
            .enable_app(&app_name)
            .await
            .expect("Failed to enable app");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_create_list_and_delete_saved_search() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        let name = unique_name("codex_saved_search");
        let _cleanup = SavedSearchCleanup::new(name.clone());

        let search = r#"| makeresults | eval foo="saved-search" | table foo"#;
        client
            .create_saved_search(&name, search)
            .await
            .expect("Failed to create saved search");

        // Retry with delay to allow Splunk to propagate the saved search
        // Splunk may take time to index the saved search in the list
        let mut created = None;
        for _attempt in 0..10 {
            // Use a high count to ensure we get all saved searches (default is 30)
            let searches = client
                .list_saved_searches(Some(1000), None)
                .await
                .expect("Failed to list saved searches");
            if let Some(s) = searches.iter().find(|s| s.name == name) {
                created = Some(s.clone());
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let created = created.expect("created saved search should be listed");
        assert_eq!(
            created.search, search,
            "created saved search should retain its search query"
        );

        client
            .delete_saved_search(&name)
            .await
            .expect("Failed to delete saved search");
    }

    #[tokio::test]
    #[ignore = "requires live Splunk server"]
    async fn test_live_get_saved_search() {
        let Some(client) = create_test_client_or_skip() else {
            return;
        };

        let name = unique_name("codex_get_saved_search");
        let _cleanup = SavedSearchCleanup::new(name.clone());

        let search = r#"| makeresults | eval foo="get-saved-search" | table foo"#;
        client
            .create_saved_search(&name, search)
            .await
            .expect("Failed to create saved search");

        let retrieved = client
            .get_saved_search(&name)
            .await
            .expect("Failed to get saved search");
        assert_eq!(retrieved.name, name);
        assert_eq!(retrieved.search, search);
    }
}
