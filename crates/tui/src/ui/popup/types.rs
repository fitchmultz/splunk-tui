//! Popup type definitions for different dialog variants.
//!
//! This module contains the `PopupType` enum which defines all possible
//! popup dialog types used throughout the TUI application.

use crate::ui::popup::ProfileField;

/// The type/kind of popup dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupType {
    /// Help dialog with keyboard shortcuts
    Help,
    /// Confirm cancel job (holds search ID)
    ConfirmCancel(String),
    /// Confirm delete job (holds search ID)
    ConfirmDelete(String),
    /// Confirm batch cancel (holds list of SIDs)
    ConfirmCancelBatch(Vec<String>),
    /// Confirm batch delete (holds list of SIDs)
    ConfirmDeleteBatch(Vec<String>),
    /// Export search results
    ExportSearch,
    /// Show full error details with structured information
    ErrorDetails,
    /// Show index details with full metadata
    IndexDetails,
    /// Confirm enable app (holds app name)
    ConfirmEnableApp(String),
    /// Confirm disable app (holds app name)
    ConfirmDisableApp(String),
    /// Profile selector popup with list of available profiles
    ProfileSelector {
        /// List of available profile names
        profiles: Vec<String>,
        /// Currently selected index
        selected_index: usize,
    },
    /// Index creation dialog
    CreateIndex {
        name_input: String,
        max_data_size_mb: Option<u64>,
        max_hot_buckets: Option<u64>,
        max_warm_db_count: Option<u64>,
        frozen_time_period_secs: Option<u64>,
        home_path: Option<String>,
        cold_db_path: Option<String>,
        thawed_path: Option<String>,
        cold_to_frozen_dir: Option<String>,
    },
    /// Index modification dialog
    ModifyIndex {
        index_name: String,
        current_max_data_size_mb: Option<u64>,
        current_max_hot_buckets: Option<u64>,
        current_max_warm_db_count: Option<u64>,
        current_frozen_time_period_secs: Option<u64>,
        current_home_path: Option<String>,
        current_cold_db_path: Option<String>,
        current_thawed_path: Option<String>,
        current_cold_to_frozen_dir: Option<String>,
        new_max_data_size_mb: Option<u64>,
        new_max_hot_buckets: Option<u64>,
        new_max_warm_db_count: Option<u64>,
        new_frozen_time_period_secs: Option<u64>,
        new_home_path: Option<String>,
        new_cold_db_path: Option<String>,
        new_thawed_path: Option<String>,
        new_cold_to_frozen_dir: Option<String>,
    },
    /// Index deletion confirmation
    DeleteIndexConfirm { index_name: String },
    /// User creation dialog
    CreateUser {
        name_input: String,
        password_input: String,
        roles_input: String,
        realname_input: String,
        email_input: String,
        default_app_input: String,
    },
    /// User modification dialog
    ModifyUser {
        user_name: String,
        current_roles: Vec<String>,
        current_realname: Option<String>,
        current_email: Option<String>,
        current_default_app: Option<String>,
        password_input: String,
        roles_input: String,
        realname_input: String,
        email_input: String,
        default_app_input: String,
    },
    /// User deletion confirmation
    DeleteUserConfirm { user_name: String },
    /// Role creation dialog
    CreateRole {
        name_input: String,
        capabilities_input: String,
        search_indexes_input: String,
        search_filter_input: String,
        imported_roles_input: String,
        default_app_input: String,
    },
    /// Role modification dialog
    ModifyRole {
        role_name: String,
        current_capabilities: Vec<String>,
        current_search_indexes: Vec<String>,
        current_search_filter: Option<String>,
        current_imported_roles: Vec<String>,
        current_default_app: Option<String>,
        capabilities_input: String,
        search_indexes_input: String,
        search_filter_input: String,
        imported_roles_input: String,
        default_app_input: String,
    },
    /// Role deletion confirmation
    DeleteRoleConfirm { role_name: String },
    /// Confirm remove app (holds app name)
    ConfirmRemoveApp(String),
    /// App installation file path input dialog
    InstallAppDialog { file_input: String },
    /// Profile creation dialog
    CreateProfile {
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    },
    /// Profile editing dialog
    EditProfile {
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    },
    /// Profile deletion confirmation
    DeleteProfileConfirm { profile_name: String },
}
