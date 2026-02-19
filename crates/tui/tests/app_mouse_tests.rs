//! Integration tests for mouse selection across all TUI list screens.
//!
//! These tests verify that mouse clicks correctly select items in all
//! list and table screens in the TUI.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use splunk_client::models::{
    App as SplunkApp, AuditAction, AuditEvent, AuditResult, Dashboard, DataModel, FiredAlert,
    Forwarder, Index, Input, InputType, LookupTable, Macro, Role, SavedSearch, SearchJobStatus,
    SearchPeer, SearchPeerStatus, ShcMember, ShcMemberStatus, User, WorkloadPool, WorkloadRule,
};
use splunk_tui::ConnectionContext;
use splunk_tui::action::Action;
use splunk_tui::app::App;
use splunk_tui::app::state::{
    ClusterViewMode, CurrentScreen, HEADER_HEIGHT, ShcViewMode, WorkloadViewMode,
};

// ============================================================================
// PlainList Screen Tests (ListState, no header)
// Data starts at HEADER_HEIGHT + 1 (row 5)
// ============================================================================

#[test]
fn test_mouse_click_selects_index_in_indexes_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![
        Index {
            name: "idx1".into(),
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
        },
        Index {
            name: "idx2".into(),
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
        },
        Index {
            name: "idx3".into(),
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
        },
    ]);

    // Click on second row (HEADER_HEIGHT + 2 = row 6, which is second data row)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(
        action.is_none(),
        "First click should just select, not trigger action"
    );
    assert_eq!(
        app.indexes_state.selected(),
        Some(1),
        "Should select index 1"
    );
}

#[test]
fn test_mouse_click_selects_app_in_apps_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Apps;
    app.apps = Some(vec![
        SplunkApp {
            name: "app1".into(),
            label: None,
            version: None,
            disabled: false,
            description: None,
            author: None,
            is_configured: None,
            is_visible: None,
        },
        SplunkApp {
            name: "app2".into(),
            label: None,
            version: None,
            disabled: false,
            description: None,
            author: None,
            is_configured: None,
            is_visible: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.apps_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_user_in_users_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Users;
    app.users = Some(vec![
        User {
            name: "user1".into(),
            realname: None,
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
        User {
            name: "user2".into(),
            realname: None,
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.users_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_role_in_roles_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Roles;
    app.roles = Some(vec![
        Role {
            name: "role1".into(),
            imported_roles: vec![],
            capabilities: vec![],
            search_filter: None,
            search_indexes: vec![],
            default_app: None,
            cumulative_srch_jobs_quota: None,
            cumulative_rt_srch_jobs_quota: None,
        },
        Role {
            name: "role2".into(),
            imported_roles: vec![],
            capabilities: vec![],
            search_filter: None,
            search_indexes: vec![],
            default_app: None,
            cumulative_srch_jobs_quota: None,
            cumulative_rt_srch_jobs_quota: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.roles_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_saved_search() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::SavedSearches;
    app.saved_searches = Some(vec![
        SavedSearch {
            name: "search1".into(),
            search: "| stats count".into(),
            description: None,
            disabled: false,
        },
        SavedSearch {
            name: "search2".into(),
            search: "| stats avg".into(),
            description: None,
            disabled: false,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.saved_searches_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_macro() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Macros;
    app.macros = Some(vec![
        Macro {
            name: "macro1".into(),
            definition: "| stats count".into(),
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
        Macro {
            name: "macro2".into(),
            definition: "| stats avg".into(),
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.macros_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_fired_alert() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::FiredAlerts;
    app.fired_alerts = Some(vec![
        FiredAlert {
            name: "alert1".into(),
            actions: None,
            alert_type: None,
            digest_mode: None,
            expiration_time_rendered: None,
            savedsearch_name: None,
            severity: None,
            sid: None,
            trigger_time: None,
            trigger_time_rendered: None,
            triggered_alerts: None,
        },
        FiredAlert {
            name: "alert2".into(),
            actions: None,
            alert_type: None,
            digest_mode: None,
            expiration_time_rendered: None,
            savedsearch_name: None,
            severity: None,
            sid: None,
            trigger_time: None,
            trigger_time_rendered: None,
            triggered_alerts: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.fired_alerts_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_dashboard() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Dashboards;
    app.dashboards = Some(vec![
        Dashboard {
            name: "dash1".into(),
            label: "Dashboard 1".into(),
            description: None,
            author: "admin".into(),
            is_dashboard: true,
            is_visible: true,
            version: None,
            xml_data: None,
            updated: None,
        },
        Dashboard {
            name: "dash2".into(),
            label: "Dashboard 2".into(),
            description: None,
            author: "admin".into(),
            is_dashboard: true,
            is_visible: true,
            version: None,
            xml_data: None,
            updated: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.dashboards_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_data_model() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::DataModels;
    app.data_models = Some(vec![
        DataModel {
            name: "dm1".into(),
            displayName: "Data Model 1".into(),
            description: None,
            owner: "admin".into(),
            app: "search".into(),
            is_accelerated: false,
            json_data: None,
            updated: None,
        },
        DataModel {
            name: "dm2".into(),
            displayName: "Data Model 2".into(),
            description: None,
            owner: "admin".into(),
            app: "search".into(),
            is_accelerated: false,
            json_data: None,
            updated: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.data_models_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_outside_list_area_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![Index {
        name: "idx1".into(),
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
    }]);
    app.indexes_state.select(Some(0));

    // Click in header area (row 2)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(
        app.indexes_state.selected(),
        Some(0),
        "Selection should not change"
    );
}

// ============================================================================
// TableWithHeader Screen Tests (TableState, has header row)
// Header at HEADER_HEIGHT + 1 (row 5), data at HEADER_HEIGHT + 2 (row 6)
// ============================================================================

#[test]
fn test_mouse_click_selects_input_in_table() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Inputs;
    app.inputs = Some(vec![
        Input {
            name: "in1".into(),
            input_type: InputType::Monitor,
            disabled: false,
            host: None,
            source: None,
            sourcetype: None,
            connection_host: None,
            port: None,
            path: None,
            blacklist: None,
            whitelist: None,
            recursive: None,
            command: None,
            interval: None,
        },
        Input {
            name: "in2".into(),
            input_type: InputType::Udp,
            disabled: false,
            host: None,
            source: None,
            sourcetype: None,
            connection_host: None,
            port: None,
            path: None,
            blacklist: None,
            whitelist: None,
            recursive: None,
            command: None,
            interval: None,
        },
    ]);

    // Table: HEADER_HEIGHT (4) + table_header (1) + data_row (1) = row 6
    // Click on second data row = row 7
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.inputs_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_on_table_header_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Inputs;
    app.inputs = Some(vec![Input {
        name: "in1".into(),
        input_type: InputType::Monitor,
        disabled: false,
        host: None,
        source: None,
        sourcetype: None,
        connection_host: None,
        port: None,
        path: None,
        blacklist: None,
        whitelist: None,
        recursive: None,
        command: None,
        interval: None,
    }]);

    // Click on table header row (HEADER_HEIGHT + 1 = row 5)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 1,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    // Header click should not change the existing selection (App::new initializes to Some(0))
    assert_eq!(app.inputs_state.selected(), Some(0));
}

#[test]
fn test_mouse_click_selects_search_peer() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::SearchPeers;
    app.search_peers = Some(vec![
        SearchPeer {
            name: "peer1".into(),
            status: SearchPeerStatus::Up,
            host: "host1".into(),
            port: 8089,
            guid: None,
            version: None,
            last_connected: None,
            disabled: None,
        },
        SearchPeer {
            name: "peer2".into(),
            status: SearchPeerStatus::Up,
            host: "host2".into(),
            port: 8089,
            guid: None,
            version: None,
            last_connected: None,
            disabled: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.search_peers_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_forwarder() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Forwarders;
    app.forwarders = Some(vec![
        Forwarder {
            name: "fwd1".into(),
            hostname: None,
            client_name: None,
            ip_address: None,
            utsname: None,
            version: None,
            last_phone: None,
            repository_location: None,
            server_classes: None,
        },
        Forwarder {
            name: "fwd2".into(),
            hostname: None,
            client_name: None,
            ip_address: None,
            utsname: None,
            version: None,
            last_phone: None,
            repository_location: None,
            server_classes: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.forwarders_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_lookup() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Lookups;
    app.lookups = Some(vec![
        LookupTable {
            name: "lookup1".into(),
            filename: "lookup1.csv".into(),
            owner: "admin".into(),
            app: "search".into(),
            sharing: "global".into(),
            size: 1024,
        },
        LookupTable {
            name: "lookup2".into(),
            filename: "lookup2.csv".into(),
            owner: "admin".into(),
            app: "search".into(),
            sharing: "global".into(),
            size: 2048,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.lookups_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_audit_event() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Audit;
    app.audit_events = Some(vec![
        AuditEvent {
            time: "2024-01-01T00:00:00Z".into(),
            user: "user1".into(),
            action: AuditAction::Login,
            target: "/".into(),
            result: AuditResult::Success,
            client_ip: "192.168.1.1".into(),
            details: "Login successful".into(),
            raw: "raw".into(),
        },
        AuditEvent {
            time: "2024-01-01T00:01:00Z".into(),
            user: "user2".into(),
            action: AuditAction::Logout,
            target: "/".into(),
            result: AuditResult::Success,
            client_ip: "192.168.1.2".into(),
            details: "Logout successful".into(),
            raw: "raw".into(),
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.audit_state.selected(), Some(1));
}

// ============================================================================
// Multi-view Screen Tests (need view_mode check)
// ============================================================================

#[test]
fn test_mouse_click_selects_cluster_peer_in_peers_view() {
    use splunk_client::models::{ClusterPeer, PeerState, PeerStatus, ReplicationStatus};

    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Cluster;
    app.cluster_view_mode = ClusterViewMode::Peers;
    app.cluster_peers = Some(vec![
        ClusterPeer {
            id: "p1".into(),
            label: None,
            status: PeerStatus::Up,
            peer_state: PeerState::Searchable,
            site: None,
            guid: "g1".into(),
            host: "h1".into(),
            port: 8080,
            replication_count: None,
            replication_status: Some(ReplicationStatus::Complete),
            bundle_replication_count: None,
            is_captain: None,
        },
        ClusterPeer {
            id: "p2".into(),
            label: None,
            status: PeerStatus::Up,
            peer_state: PeerState::Searchable,
            site: None,
            guid: "g2".into(),
            host: "h2".into(),
            port: 8080,
            replication_count: None,
            replication_status: Some(ReplicationStatus::Complete),
            bundle_replication_count: None,
            is_captain: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.cluster_peers_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_in_cluster_summary_view_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Cluster;
    app.cluster_view_mode = ClusterViewMode::Summary;
    // Store the initial selection (App::new initializes to Some(0))
    let initial_selection = app.cluster_peers_state.selected();

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    // In summary view, selection should not change from initial value
    assert_eq!(app.cluster_peers_state.selected(), initial_selection);
}

#[test]
fn test_mouse_click_selects_workload_pool() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::WorkloadManagement;
    app.workload_view_mode = WorkloadViewMode::Pools;
    app.workload_pools = Some(vec![
        WorkloadPool {
            name: "pool1".into(),
            cpu_weight: None,
            mem_weight: None,
            default_pool: None,
            enabled: None,
            search_concurrency: None,
            search_time_range: None,
            admission_rules_enabled: None,
            cpu_cores: None,
            mem_limit: None,
        },
        WorkloadPool {
            name: "pool2".into(),
            cpu_weight: None,
            mem_weight: None,
            default_pool: None,
            enabled: None,
            search_concurrency: None,
            search_time_range: None,
            admission_rules_enabled: None,
            cpu_cores: None,
            mem_limit: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.workload_pools_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_workload_rule() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::WorkloadManagement;
    app.workload_view_mode = WorkloadViewMode::Rules;
    app.workload_rules = Some(vec![
        WorkloadRule {
            name: "rule1".into(),
            predicate: None,
            workload_pool: None,
            user: None,
            app: None,
            search_type: None,
            search_time_range: None,
            enabled: None,
            order: None,
        },
        WorkloadRule {
            name: "rule2".into(),
            predicate: None,
            workload_pool: None,
            user: None,
            app: None,
            search_type: None,
            search_time_range: None,
            enabled: None,
            order: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.workload_rules_state.selected(), Some(1));
}

#[test]
fn test_mouse_click_selects_shc_member() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Shc;
    app.shc_view_mode = ShcViewMode::Members;
    app.shc_members = Some(vec![
        ShcMember {
            id: "member1".into(),
            label: None,
            host: "host1".into(),
            port: 8080,
            status: ShcMemberStatus::Up,
            is_captain: false,
            is_dynamic_captain: None,
            guid: "guid1".into(),
            site: None,
            replication_port: None,
            last_heartbeat: None,
            pending_job_count: None,
        },
        ShcMember {
            id: "member2".into(),
            label: None,
            host: "host2".into(),
            port: 8080,
            status: ShcMemberStatus::Up,
            is_captain: false,
            is_dynamic_captain: None,
            guid: "guid2".into(),
            site: None,
            replication_port: None,
            last_heartbeat: None,
            pending_job_count: None,
        },
    ]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 3,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert_eq!(app.shc_members_state.selected(), Some(1));
}

// ============================================================================
// Double-click Activation Tests
// ============================================================================

/// NOTE: Jobs double-click tests rely on filtered_jobs_len() which requires
/// calling rebuild_filtered_indices(). This is pub(crate) so we can't call it
/// from integration tests. These tests are covered by unit tests in mouse.rs.
/// See: test_handle_mouse_content_click_jobs in crates/tui/src/app/mouse.rs

#[test]
fn test_double_click_on_different_job_does_not_trigger_inspect() {
    // This test verifies that clicking a different row doesn't trigger inspect
    // even without filtered indices being set up
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(vec![
        SearchJobStatus {
            sid: "job1".into(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 0,
            event_count: 0,
            result_count: 0,
            disk_usage: 0,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "job2".into(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 0,
            event_count: 0,
            result_count: 0,
            disk_usage: 0,
            priority: None,
            label: None,
        },
    ]);

    // Without filtered indices, clicks don't select items
    // This is expected behavior - the test just verifies no panic occurs
    let event1 = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 1,
        modifiers: KeyModifiers::empty(),
    };
    let action1 = app.handle_mouse(event1);
    assert!(
        action1.is_none(),
        "Without filtered indices, click should not trigger action"
    );

    let event2 = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 2,
        modifiers: KeyModifiers::empty(),
    };
    let action2 = app.handle_mouse(event2);
    assert!(
        action2.is_none(),
        "Clicking different row should not trigger inspect"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_mouse_click_during_loading_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Apps;
    app.loading = true;
    // Deselect first (App::new initializes to Some(0))
    app.apps_state.select(None);
    app.apps = Some(vec![SplunkApp {
        name: "app1".into(),
        label: None,
        version: None,
        disabled: false,
        description: None,
        author: None,
        is_configured: None,
        is_visible: None,
    }]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 1,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    assert!(
        app.apps_state.selected().is_none(),
        "Should not select while loading"
    );
}

#[test]
fn test_mouse_click_with_popup_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Apps;
    app.popup = Some(
        splunk_tui::ui::popup::Popup::builder(splunk_tui::ui::popup::PopupType::ConfirmCancel(
            "test-job-123".into(),
        ))
        .build(),
    );
    app.apps = Some(vec![SplunkApp {
        name: "app1".into(),
        label: None,
        version: None,
        disabled: false,
        description: None,
        author: None,
        is_configured: None,
        is_visible: None,
    }]);

    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 1,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
}

#[test]
fn test_mouse_click_beyond_data_does_not_select() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 80, 24);
    app.current_screen = CurrentScreen::Apps;
    // Deselect first (App::new initializes to Some(0))
    app.apps_state.select(None);
    app.apps = Some(vec![SplunkApp {
        name: "app1".into(),
        label: None,
        version: None,
        disabled: false,
        description: None,
        author: None,
        is_configured: None,
        is_visible: None,
    }]);

    // Click way below data area
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_HEIGHT + 10,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(action.is_none());
    // Should remain deselected since click was beyond data
    assert!(app.apps_state.selected().is_none());
}

// ============================================================================
// Scroll Wheel Tests
// ============================================================================

#[test]
fn test_scroll_up_returns_navigate_up_action() {
    let mut app = App::new(None, ConnectionContext::default());

    let event = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(matches!(action, Some(Action::NavigateUp)));
}

#[test]
fn test_scroll_down_returns_navigate_down_action() {
    let mut app = App::new(None, ConnectionContext::default());

    let event = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };

    let action = app.handle_mouse(event);
    assert!(matches!(action, Some(Action::NavigateDown)));
}
