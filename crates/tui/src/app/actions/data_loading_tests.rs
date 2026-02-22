//! Purpose: Regression and behavior tests for TUI data-loading action handlers.
//! Responsibilities: Verify state transitions for successful/failed load actions and domain-specific side effects.
//! Non-scope: Does not perform network calls; all actions are injected directly.
//! Invariants/Assumptions: Tests remain deterministic and operate on in-memory app state only.

use super::*;
use crate::ConnectionContext;
use crate::app::state::HealthState;
use splunk_client::models::{HealthCheckOutput, HealthStatus, SplunkHealth};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_indexes_loaded_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());

    let indexes = vec![splunk_client::models::Index {
        name: "test_index".to_string(),
        max_total_data_size_mb: None,
        current_db_size_mb: 0,
        total_event_count: 0,
        max_warm_db_count: None,
        max_hot_buckets: None,
        frozen_time_period_in_secs: None,
        cold_db_path: None,
        home_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
        primary_index: None,
    }];

    app.handle_data_loading_action(Action::IndexesLoaded(Ok(indexes)));

    assert!(app.indexes.is_some());
    assert_eq!(app.indexes.as_ref().unwrap().len(), 1);
    assert!(!app.loading);
}

#[test]
fn test_health_status_loaded_ok() {
    let mut app = App::new(None, ConnectionContext::default());

    let health = SplunkHealth {
        health: HealthStatus::Green,
        features: HashMap::new(),
    };

    app.handle_data_loading_action(Action::HealthStatusLoaded(Ok(health)));

    assert_eq!(app.health_state, HealthState::Healthy);
}

#[test]
fn test_health_status_loaded_err() {
    let mut app = App::new(None, ConnectionContext::default());
    app.health_state = HealthState::Healthy;

    let error = splunk_client::ClientError::ConnectionRefused("test".to_string());
    app.handle_data_loading_action(Action::HealthStatusLoaded(Err(Arc::new(error))));

    assert_eq!(app.health_state, HealthState::Unhealthy);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn test_health_loaded_with_splunkd_health() {
    let mut app = App::new(None, ConnectionContext::default());

    let health_output = HealthCheckOutput {
        server_info: None,
        splunkd_health: Some(SplunkHealth {
            health: HealthStatus::Red,
            features: HashMap::new(),
        }),
        license_usage: None,
        kvstore_status: None,
        log_parsing_health: None,
        circuit_breaker_states: None,
    };

    app.handle_data_loading_action(Action::HealthLoaded(Box::new(Ok(health_output))));

    assert_eq!(app.health_state, HealthState::Unhealthy);
}

#[test]
fn test_jobs_loaded_preserves_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.jobs_state.select(Some(5));

    let jobs = vec![
        splunk_client::SearchJobStatus {
            sid: "job1".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            disk_usage: 1024,
            priority: None,
            label: None,
        },
        splunk_client::SearchJobStatus {
            sid: "job2".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 2.0,
            cursor_time: None,
            scan_count: 200,
            event_count: 100,
            result_count: 50,
            disk_usage: 2048,
            priority: None,
            label: None,
        },
    ];

    app.handle_data_loading_action(Action::JobsLoaded(Ok(jobs)));

    assert!(app.jobs.is_some());
    // Selection should be clamped to new bounds (2 jobs, so max index is 1)
    assert_eq!(app.jobs_state.selected(), Some(1));
}

#[test]
fn test_data_load_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::IndexesLoaded(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
}

#[test]
fn test_cluster_info_not_found_is_treated_as_expected_unclustered_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.cluster_info = Some(splunk_client::models::ClusterInfo {
        id: "manager-1".to_string(),
        label: Some("cluster".to_string()),
        mode: splunk_client::models::ClusterMode::Manager,
        manager_uri: Some("https://example:8089".to_string()),
        replication_factor: Some(2),
        search_factor: Some(2),
        status: Some(splunk_client::models::ClusterStatus::Enabled),
        maintenance_mode: Some(false),
    });

    app.handle_data_loading_action(Action::ClusterInfoLoaded(Err(Arc::new(
        splunk_client::ClientError::NotFound("cluster manager endpoint".to_string()),
    ))));

    assert!(app.cluster_info.is_none());
    assert!(app.current_error.is_none());
    assert!(app.toasts.is_empty());
    assert!(!app.loading);
}

#[test]
fn test_cluster_peers_404_is_treated_as_expected_unclustered_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.cluster_peers = Some(vec![splunk_client::models::ClusterPeer {
        id: "peer-1".to_string(),
        label: Some("peer".to_string()),
        status: splunk_client::models::PeerStatus::Up,
        peer_state: splunk_client::models::PeerState::Searchable,
        site: Some("site1".to_string()),
        guid: "guid-1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        replication_count: Some(1),
        replication_status: Some(splunk_client::models::ReplicationStatus::Complete),
        bundle_replication_count: Some(1),
        is_captain: Some(false),
    }]);

    app.handle_data_loading_action(Action::ClusterPeersLoaded(Err(Arc::new(
        splunk_client::ClientError::ApiError {
            status: 404,
            url: "/services/cluster/master/peers".to_string(),
            message: "not found".to_string(),
            request_id: None,
        },
    ))));

    assert!(app.cluster_peers.is_none());
    assert!(app.current_error.is_none());
    assert!(app.toasts.is_empty());
    assert!(!app.loading);
}

#[test]
fn test_shc_status_loaded_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.shc_unavailable = true;

    app.handle_data_loading_action(Action::ShcStatusLoaded(Ok(
        splunk_client::models::ShcStatus {
            is_captain: false,
            is_searchable: true,
            captain_uri: Some("https://example:8089".to_string()),
            member_count: 1,
            minimum_member_count: Some(1),
            election_timeout: Some(60),
            rolling_restart_flag: Some(false),
            service_ready_flag: Some(true),
        },
    )));

    assert!(app.shc_status.is_some());
    assert!(!app.shc_unavailable);
    assert!(!app.loading);
}

#[test]
fn test_shc_status_404_is_treated_as_expected_unclustered_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.shc_status = Some(splunk_client::models::ShcStatus {
        is_captain: true,
        is_searchable: true,
        captain_uri: Some("https://example:8089".to_string()),
        member_count: 3,
        minimum_member_count: Some(2),
        election_timeout: Some(60),
        rolling_restart_flag: Some(false),
        service_ready_flag: Some(true),
    });

    app.handle_data_loading_action(Action::ShcStatusLoaded(Err(Arc::new(
        splunk_client::ClientError::ApiError {
            status: 404,
            url: "/services/shcluster/member/info".to_string(),
            message: "not found".to_string(),
            request_id: None,
        },
    ))));

    assert!(app.shc_status.is_none());
    assert!(app.shc_unavailable);
    assert!(app.current_error.is_none());
    assert!(app.toasts.is_empty());
    assert!(!app.loading);
}

#[test]
fn test_shc_status_503_from_shcluster_endpoint_is_treated_as_expected_unclustered_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.shc_status = Some(splunk_client::models::ShcStatus {
        is_captain: true,
        is_searchable: true,
        captain_uri: Some("https://example:8089".to_string()),
        member_count: 3,
        minimum_member_count: Some(2),
        election_timeout: Some(60),
        rolling_restart_flag: Some(false),
        service_ready_flag: Some(true),
    });

    app.handle_data_loading_action(Action::ShcStatusLoaded(Err(Arc::new(
        splunk_client::ClientError::ApiError {
            status: 503,
            url: "/services/shcluster/member/info".to_string(),
            message: "Service temporarily unavailable".to_string(),
            request_id: None,
        },
    ))));

    assert!(app.shc_status.is_none());
    assert!(app.shc_unavailable);
    assert!(app.current_error.is_none());
    assert!(app.toasts.is_empty());
    assert!(!app.loading);
}

#[test]
fn test_cluster_info_503_is_not_treated_as_expected_unclustered_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    app.handle_data_loading_action(Action::ClusterInfoLoaded(Err(Arc::new(
        splunk_client::ClientError::ApiError {
            status: 503,
            url: "/services/cluster/manager/info".to_string(),
            message: "Service temporarily unavailable".to_string(),
            request_id: None,
        },
    ))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
}

#[test]
fn test_config_files_loaded_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let files = vec![
        splunk_client::models::ConfigFile {
            name: "props".to_string(),
            title: "props.conf".to_string(),
            description: Some("Properties configuration".to_string()),
        },
        splunk_client::models::ConfigFile {
            name: "transforms".to_string(),
            title: "transforms.conf".to_string(),
            description: Some("Transformations".to_string()),
        },
    ];

    app.handle_data_loading_action(Action::ConfigFilesLoaded(Ok(files)));

    assert!(app.config_files.is_some());
    assert_eq!(app.config_files.as_ref().unwrap().len(), 2);
    assert!(!app.loading);
}

#[test]
fn test_config_files_loaded_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::ConfigFilesLoaded(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("config files"));
}

#[test]
fn test_config_stanzas_loaded_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let stanzas = vec![
        splunk_client::models::ConfigStanza {
            name: "default".to_string(),
            config_file: "props".to_string(),
            settings: std::collections::HashMap::new(),
        },
        splunk_client::models::ConfigStanza {
            name: "access_combined".to_string(),
            config_file: "props".to_string(),
            settings: std::collections::HashMap::new(),
        },
    ];

    app.handle_data_loading_action(Action::ConfigStanzasLoaded(Ok(stanzas)));

    assert!(app.config_stanzas.is_some());
    assert_eq!(app.config_stanzas.as_ref().unwrap().len(), 2);
    assert!(!app.loading);
    // filtered_stanza_indices should be rebuilt
    assert_eq!(app.filtered_stanza_indices.len(), 2);
}

#[test]
fn test_config_stanzas_loaded_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::ConfigStanzasLoaded(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("config stanzas"));
}

// Macro action handler tests
#[test]
fn test_macros_loaded_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let macros = vec![
        splunk_client::models::Macro {
            name: "test_macro".to_string(),
            definition: "index=main | head 10".to_string(),
            args: None,
            description: Some("Test macro".to_string()),
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
        splunk_client::models::Macro {
            name: "param_macro(2)".to_string(),
            definition: "index=$arg1$ | head $arg2$".to_string(),
            args: Some("arg1,arg2".to_string()),
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
    ];

    app.handle_data_loading_action(Action::MacrosLoaded(Ok(macros)));

    assert!(app.macros.is_some());
    assert_eq!(app.macros.as_ref().unwrap().len(), 2);
    assert!(!app.loading);
}

#[test]
fn test_macros_loaded_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::MacrosLoaded(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("macros"));
}

#[test]
fn test_macro_created_success_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    app.handle_data_loading_action(Action::MacroCreated(Ok(())));

    assert!(!app.loading);
    assert_eq!(app.toasts.len(), 1);
    assert!(app.toasts[0].message.contains("created"));
}

#[test]
fn test_macro_created_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::MacroCreated(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("create macro"));
}

#[test]
fn test_macro_updated_success_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    app.handle_data_loading_action(Action::MacroUpdated(Ok(())));

    assert!(!app.loading);
    assert_eq!(app.toasts.len(), 1);
    assert!(app.toasts[0].message.contains("updated"));
}

#[test]
fn test_macro_updated_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::MacroUpdated(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("update macro"));
}

#[test]
fn test_macro_deleted_success_removes_from_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;
    app.macros = Some(vec![
        splunk_client::models::Macro {
            name: "macro_to_delete".to_string(),
            definition: "index=main".to_string(),
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
        splunk_client::models::Macro {
            name: "keep_this_macro".to_string(),
            definition: "index=internal".to_string(),
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
    ]);

    app.handle_data_loading_action(Action::MacroDeleted(Ok("macro_to_delete".to_string())));

    assert!(!app.loading);
    assert_eq!(app.toasts.len(), 1);
    assert!(app.toasts[0].message.contains("deleted"));
    // Verify the macro was removed from the list
    assert_eq!(app.macros.as_ref().unwrap().len(), 1);
    assert_eq!(app.macros.as_ref().unwrap()[0].name, "keep_this_macro");
}

#[test]
fn test_macro_deleted_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.handle_data_loading_action(Action::MacroDeleted(Err(Arc::new(error))));

    assert!(app.current_error.is_some());
    assert_eq!(app.toasts.len(), 1);
    assert!(!app.loading);
    assert!(app.toasts[0].message.contains("delete macro"));
}

#[test]
fn test_settings_loaded_updates_search_defaults_and_page_size() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    // Verify initial state (default max_results = 1000)
    assert_eq!(app.search_defaults.max_results, 1000);
    assert_eq!(app.search_results_page_size, 1000);

    // Create new persisted state with different search_defaults
    let new_state = splunk_config::PersistedState {
        search_defaults: splunk_config::SearchDefaults {
            max_results: 500,
            earliest_time: "-48h".to_string(),
            latest_time: "now".to_string(),
        },
        auto_refresh: true,
        sort_column: "sid".to_string(),
        sort_direction: "asc".to_string(),
        last_search_query: Some("test query".to_string()),
        search_history: vec!["query1".to_string()],
        selected_theme: splunk_config::ColorTheme::Default,
        keybind_overrides: splunk_config::KeybindOverrides::default(),
        list_defaults: splunk_config::ListDefaults::default(),
        internal_logs_defaults: splunk_config::InternalLogsDefaults::default(),
        tutorial_completed: false,
        current_screen: "Search".to_string(),
        scroll_positions: splunk_config::ScrollPositions::default(),
        recent_export_paths: Vec::new(),
        export_format: "Json".to_string(),
        last_saved_at: None,
        onboarding_checklist: splunk_config::PersistedOnboardingChecklist::default(),
    };

    app.handle_data_loading_action(Action::SettingsLoaded(new_state));

    // Verify search_defaults was updated
    assert_eq!(app.search_defaults.max_results, 500);
    assert_eq!(app.search_defaults.earliest_time, "-48h");

    // Verify search_results_page_size was synced to new max_results
    assert_eq!(app.search_results_page_size, 500);

    // Verify other fields were updated
    assert!(app.auto_refresh);
    assert_eq!(app.search_input.value(), "test query");
    assert!(!app.loading);
}

#[test]
fn test_settings_loaded_handles_zero_max_results() {
    let mut app = App::new(None, crate::ConnectionContext::default());
    app.loading = true;

    // Create persisted state with max_results = 0 (invalid, should default to 1000)
    let new_state = splunk_config::PersistedState {
        search_defaults: splunk_config::SearchDefaults {
            max_results: 0,
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
        },
        auto_refresh: false,
        sort_column: "sid".to_string(),
        sort_direction: "asc".to_string(),
        last_search_query: None,
        search_history: vec![],
        selected_theme: splunk_config::ColorTheme::Default,
        keybind_overrides: splunk_config::KeybindOverrides::default(),
        list_defaults: splunk_config::ListDefaults::default(),
        internal_logs_defaults: splunk_config::InternalLogsDefaults::default(),
        tutorial_completed: false,
        current_screen: "Search".to_string(),
        scroll_positions: splunk_config::ScrollPositions::default(),
        recent_export_paths: Vec::new(),
        export_format: "Json".to_string(),
        last_saved_at: None,
        onboarding_checklist: splunk_config::PersistedOnboardingChecklist::default(),
    };

    app.handle_data_loading_action(Action::SettingsLoaded(new_state));

    // Verify search_defaults.max_results was set to 0 (raw value stored)
    assert_eq!(app.search_defaults.max_results, 0);

    // But search_results_page_size should default to 1000 (validation applied)
    assert_eq!(app.search_results_page_size, 1000);
}

#[test]
fn test_multi_instance_instance_loaded_incremental() {
    let mut app = App::new(None, crate::ConnectionContext::default());
    use crate::action::{InstanceOverview, InstanceStatus};

    let instance1 = InstanceOverview {
        profile_name: "prod".to_string(),
        base_url: "url1".to_string(),
        resources: vec![],
        error: None,
        health_status: "green".to_string(),
        job_count: 5,
        status: InstanceStatus::Healthy,
        last_success_at: None,
    };

    app.handle_data_loading_action(Action::MultiInstanceInstanceLoaded(instance1.clone()));

    assert!(app.multi_instance_data.is_some());
    assert_eq!(app.multi_instance_data.as_ref().unwrap().instances.len(), 1);
    assert_eq!(
        app.multi_instance_data.as_ref().unwrap().instances[0].profile_name,
        "prod"
    );
    assert_eq!(
        app.multi_instance_data.as_ref().unwrap().instances[0].status,
        InstanceStatus::Healthy
    );

    // Load second instance
    let instance2 = InstanceOverview {
        profile_name: "dev".to_string(),
        base_url: "url2".to_string(),
        resources: vec![],
        error: None,
        health_status: "green".to_string(),
        job_count: 1,
        status: InstanceStatus::Healthy,
        last_success_at: None,
    };

    app.handle_data_loading_action(Action::MultiInstanceInstanceLoaded(instance2));
    assert_eq!(app.multi_instance_data.as_ref().unwrap().instances.len(), 2);
}

#[test]
fn test_multi_instance_cached_fallback() {
    let mut app = App::new(None, crate::ConnectionContext::default());
    use crate::action::{InstanceOverview, InstanceStatus, OverviewResource};

    // 1. Load healthy data
    let healthy = InstanceOverview {
        profile_name: "prod".to_string(),
        base_url: "url1".to_string(),
        resources: vec![OverviewResource {
            resource_type: "jobs".to_string(),
            count: 10,
            status: "ok".to_string(),
            error: None,
        }],
        error: None,
        health_status: "green".to_string(),
        job_count: 10,
        status: InstanceStatus::Healthy,
        last_success_at: None,
    };
    app.handle_data_loading_action(Action::MultiInstanceInstanceLoaded(healthy));

    // 2. Load failing data for same instance
    let failing = InstanceOverview {
        profile_name: "prod".to_string(),
        base_url: "url1".to_string(),
        resources: vec![],
        error: Some("Connection timed out".to_string()),
        health_status: "error".to_string(),
        job_count: 0,
        status: InstanceStatus::Failed,
        last_success_at: None,
    };
    app.handle_data_loading_action(Action::MultiInstanceInstanceLoaded(failing));

    let data = app.multi_instance_data.as_ref().unwrap();
    let instance = &data.instances[0];

    // Should be Cached, not Failed
    assert_eq!(instance.status, InstanceStatus::Cached);
    // Should preserve old resources
    assert_eq!(instance.resources.len(), 1);
    assert_eq!(instance.job_count, 10);
    // Should show the new error
    assert_eq!(instance.error, Some("Connection timed out".to_string()));
}
