//! Popup type definitions for different dialog variants.
//!
//! This module contains the `PopupType` enum which defines all possible
//! popup dialog types used throughout the TUI application.

use crate::onboarding::TutorialState;
use crate::ui::popup::{MacroField, ProfileField, SavedSearchField};

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
        max_data_size_mb: Option<usize>,
        max_hot_buckets: Option<usize>,
        max_warm_db_count: Option<usize>,
        frozen_time_period_secs: Option<usize>,
        home_path: Option<String>,
        cold_db_path: Option<String>,
        thawed_path: Option<String>,
        cold_to_frozen_dir: Option<String>,
    },
    /// Index modification dialog
    ModifyIndex {
        index_name: String,
        current_max_data_size_mb: Option<usize>,
        current_max_hot_buckets: Option<usize>,
        current_max_warm_db_count: Option<usize>,
        current_frozen_time_period_secs: Option<usize>,
        current_home_path: Option<String>,
        current_cold_db_path: Option<String>,
        current_thawed_path: Option<String>,
        current_cold_to_frozen_dir: Option<String>,
        new_max_data_size_mb: Option<usize>,
        new_max_hot_buckets: Option<usize>,
        new_max_warm_db_count: Option<usize>,
        new_frozen_time_period_secs: Option<usize>,
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
        /// Whether this dialog was opened from the tutorial
        from_tutorial: bool,
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
    /// Edit saved search dialog
    EditSavedSearch {
        /// Name of the saved search being edited
        search_name: String,
        /// Current search query input
        search_input: String,
        /// Current description input
        description_input: String,
        /// Disabled toggle state
        disabled: bool,
        /// Currently selected field for navigation
        selected_field: SavedSearchField,
    },
    /// Create saved search dialog
    CreateSavedSearch {
        /// Name input
        name_input: String,
        /// Search query input
        search_input: String,
        /// Description input
        description_input: String,
        /// Disabled toggle
        disabled: bool,
        /// Currently selected field for navigation
        selected_field: SavedSearchField,
    },
    /// Delete saved search confirmation
    DeleteSavedSearchConfirm { search_name: String },
    /// Delete lookup table confirmation
    DeleteLookupConfirm { lookup_name: String },
    /// Create macro dialog
    CreateMacro {
        /// Name input
        name_input: String,
        /// Definition input (the SPL expression)
        definition_input: String,
        /// Arguments input (comma-separated list)
        args_input: String,
        /// Description input
        description_input: String,
        /// Disabled toggle
        disabled: bool,
        /// IsEval toggle (whether definition is an eval expression)
        iseval: bool,
        /// Currently selected field for navigation
        selected_field: MacroField,
    },
    /// Edit macro dialog
    EditMacro {
        /// Name of the macro being edited (display only, not editable)
        macro_name: String,
        /// Definition input (the SPL expression) - changes only applied if non-empty
        definition_input: String,
        /// Arguments input (comma-separated list) - changes only applied if non-empty
        args_input: String,
        /// Description input - changes only applied if non-empty
        description_input: String,
        /// Disabled toggle
        disabled: bool,
        /// IsEval toggle (whether definition is an eval expression)
        iseval: bool,
        /// Currently selected field for navigation
        selected_field: MacroField,
    },
    /// Tutorial wizard for first-run onboarding
    TutorialWizard {
        /// Current tutorial state
        state: TutorialState,
    },
    /// Command palette for quick navigation and action execution
    CommandPalette {
        /// Current search input
        input: String,
        /// Currently selected index in filtered results
        selected_index: usize,
        /// Filtered command items (cached from fuzzy search)
        filtered_items: Vec<crate::app::command_palette::CommandPaletteItem>,
    },
    /// Undo history viewer showing recent operations
    UndoHistory {
        /// Current scroll offset for viewing history
        scroll_offset: usize,
    },
}
