//! Popup struct and builder implementation.
//!
//! This module provides the `Popup` struct and `PopupBuilder` for constructing
//! popup dialogs with customizable titles and content.

use crate::action::variants::{ConnectionDiagnosticsResult, DiagnosticStatus};
use crate::app::App;
use crate::error_details::AuthRecoveryKind;
use crate::input::help;
use crate::onboarding::{TutorialState, TutorialSteps};
use crate::ui::popup::{MacroField, PopupType, ProfileField, SavedSearchField};

/// A modal popup dialog with title, content, and type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Popup {
    /// The title displayed in the popup border
    pub title: String,
    /// The main content text of the popup
    pub content: String,
    /// The kind/type of popup (determines behavior and default styling)
    pub kind: PopupType,
}

impl Popup {
    /// Create a new `PopupBuilder` for the given popup type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use splunk_tui::ui::popup::{Popup, PopupType};
    ///
    /// let popup = Popup::builder(PopupType::Help).build();
    /// assert_eq!(popup.title, "Help");
    /// ```
    pub fn builder(kind: PopupType) -> PopupBuilder {
        PopupBuilder::new(kind)
    }
}

/// Builder for constructing `Popup` instances.
pub struct PopupBuilder {
    kind: PopupType,
    title: Option<String>,
    content: Option<String>,
}

impl PopupBuilder {
    /// Create a new builder for the given popup type.
    pub fn new(kind: PopupType) -> Self {
        Self {
            kind,
            title: None,
            content: None,
        }
    }

    /// Set the popup title.
    ///
    /// If not set, a default title will be used based on the popup type.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the popup content.
    ///
    /// If not set, default content will be used based on the popup type.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Build the `Popup` instance using defaults derived from `PopupType`.
    pub fn build(self) -> Popup {
        let (default_title, default_content) = self.build_defaults();

        Popup {
            title: self.title.unwrap_or(default_title),
            content: self.content.unwrap_or(default_content),
            kind: self.kind,
        }
    }

    /// Build the `Popup` instance with app context for context-aware popups.
    ///
    /// This method provides the app state to popup types that need contextual
    /// information (like Help popups that show screen-specific keybindings).
    pub fn build_with_context(self, app: &App) -> Popup {
        let (default_title, default_content) = self.build_defaults_with_context(app);

        Popup {
            title: self.title.unwrap_or(default_title),
            content: self.content.unwrap_or(default_content),
            kind: self.kind,
        }
    }

    fn build_defaults(&self) -> (String, String) {
        match &self.kind {
            PopupType::Help => ("Help".to_string(), help::help_text()),
            // Other popup types that don't need context
            PopupType::ConfirmCancel(sid) => (
                "Confirm Cancel".to_string(),
                format!("Cancel job {sid}? (y/n)"),
            ),
            PopupType::ConfirmDelete(sid) => (
                "Confirm Delete".to_string(),
                format!("Delete job {sid}? (y/n)"),
            ),
            PopupType::ConfirmCancelBatch(sids) => (
                "Confirm Batch Cancel".to_string(),
                format!("Cancel {} job(s)? (y/n)", sids.len()),
            ),
            PopupType::ConfirmDeleteBatch(sids) => (
                "Confirm Batch Delete".to_string(),
                format!("Delete {} job(s)? (y/n)", sids.len()),
            ),
            PopupType::ExportSearch => (
                "Export".to_string(),
                "Enter filename: export.json\nFormat: JSON (Tab to toggle)".to_string(),
            ),
            PopupType::ErrorDetails => (
                "Error Details".to_string(),
                "Press Esc, q, or e to close".to_string(),
            ),
            PopupType::IndexDetails => (
                "Index Details".to_string(),
                "Press Esc or q to close, j/k to scroll".to_string(),
            ),
            PopupType::ConfirmEnableApp(name) => (
                "Confirm Enable".to_string(),
                format!("Enable app '{}'? (y/n)", name),
            ),
            PopupType::ConfirmDisableApp(name) => (
                "Confirm Disable".to_string(),
                format!("Disable app '{}'? (y/n)", name),
            ),
            PopupType::ProfileSelector {
                profiles,
                selected_index,
            } => self.build_profile_selector_defaults(profiles, *selected_index),
            PopupType::CreateIndex { name_input, .. } => (
                "Create Index".to_string(),
                format!(
                    "Create new index:\n\nName: {}\n\nEnter name and press Enter to create, Esc to cancel",
                    name_input
                ),
            ),
            PopupType::ModifyIndex { index_name, .. } => (
                "Modify Index".to_string(),
                format!(
                    "Modify index '{}':\n\nPress Enter to apply changes, Esc to cancel",
                    index_name
                ),
            ),
            PopupType::DeleteIndexConfirm { index_name } => (
                "Confirm Delete".to_string(),
                format!(
                    "Delete index '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    index_name
                ),
            ),
            PopupType::CreateUser { name_input, .. } => (
                "Create User".to_string(),
                format!(
                    "Create new user:\n\nName: {}\n\nEnter name and press Enter to create, Esc to cancel",
                    name_input
                ),
            ),
            PopupType::ModifyUser { user_name, .. } => (
                "Modify User".to_string(),
                format!(
                    "Modify user '{}':\n\nPress Enter to apply changes, Esc to cancel",
                    user_name
                ),
            ),
            PopupType::DeleteUserConfirm { user_name } => (
                "Confirm Delete".to_string(),
                format!(
                    "Delete user '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    user_name
                ),
            ),
            PopupType::CreateRole { name_input, .. } => (
                "Create Role".to_string(),
                format!(
                    "Create new role:\n\nName: {}\n\nEnter name and press Enter to create, Esc to cancel",
                    name_input
                ),
            ),
            PopupType::ModifyRole { role_name, .. } => (
                "Modify Role".to_string(),
                format!(
                    "Modify role '{}':\n\nPress Enter to apply changes, Esc to cancel",
                    role_name
                ),
            ),
            PopupType::DeleteRoleConfirm { role_name } => (
                "Confirm Delete".to_string(),
                format!(
                    "Delete role '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    role_name
                ),
            ),
            PopupType::ConfirmRemoveApp(app_name) => (
                "Confirm Remove App".to_string(),
                format!(
                    "Remove app '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    app_name
                ),
            ),
            PopupType::InstallAppDialog { file_input } => (
                "Install App".to_string(),
                format!(
                    "Enter path to .spl file:\n\n{}\n\nPress Enter to install, Esc to cancel",
                    file_input
                ),
            ),
            PopupType::CreateProfile {
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
                ..
            } => self.build_create_profile_defaults(
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
            ),
            PopupType::EditProfile {
                original_name,
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            } => self.build_edit_profile_defaults(
                original_name,
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
            ),
            PopupType::DeleteProfileConfirm { profile_name } => (
                "Confirm Delete Profile".to_string(),
                format!(
                    "Delete profile '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    profile_name
                ),
            ),
            PopupType::EditSavedSearch {
                search_name,
                search_input,
                description_input,
                disabled,
                selected_field,
            } => self.build_edit_saved_search_defaults(
                search_name,
                search_input,
                description_input,
                *disabled,
                *selected_field,
            ),
            PopupType::CreateSavedSearch {
                name_input,
                search_input,
                description_input,
                disabled,
                selected_field,
            } => self.build_create_saved_search_defaults(
                name_input,
                search_input,
                description_input,
                *disabled,
                *selected_field,
            ),
            PopupType::DeleteSavedSearchConfirm { search_name } => (
                "Confirm Delete".to_string(),
                format!(
                    "Delete saved search '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    search_name
                ),
            ),
            PopupType::DeleteLookupConfirm { lookup_name } => (
                "Confirm Delete".to_string(),
                format!(
                    "Delete lookup '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    lookup_name
                ),
            ),
            PopupType::CreateMacro {
                name_input,
                definition_input,
                args_input,
                description_input,
                disabled,
                iseval,
                selected_field,
            } => self.build_create_macro_defaults(
                name_input,
                definition_input,
                args_input,
                description_input,
                *disabled,
                *iseval,
                *selected_field,
            ),
            PopupType::EditMacro {
                macro_name,
                definition_input,
                args_input,
                description_input,
                disabled,
                iseval,
                selected_field,
            } => self.build_edit_macro_defaults(
                macro_name,
                definition_input,
                args_input,
                description_input,
                *disabled,
                *iseval,
                *selected_field,
            ),
            PopupType::TutorialWizard { state } => self.build_tutorial_wizard_defaults(state),
            PopupType::CommandPalette {
                input,
                selected_index,
                filtered_items,
            } => self.build_command_palette_defaults(input, *selected_index, filtered_items),
            PopupType::UndoHistory { .. } => (
                "Undo History".to_string(),
                "Recent operations (press Esc to close, j/k to scroll)".to_string(),
            ),
            PopupType::AuthRecovery { kind } => self.build_auth_recovery_defaults(kind),
            PopupType::ConnectionDiagnostics { result } => {
                self.build_connection_diagnostics_defaults(result)
            }
        }
    }

    fn build_defaults_with_context(&self, app: &App) -> (String, String) {
        match &self.kind {
            PopupType::Help => {
                let input_mode = match app.current_screen {
                    crate::app::state::CurrentScreen::Search => Some(app.search_input_mode),
                    _ => None,
                };
                (
                    format!("Help - {:?}", app.current_screen),
                    help::contextual_help_text(app.current_screen, input_mode),
                )
            }
            // All other popup types use the context-free defaults
            _ => self.build_defaults(),
        }
    }

    fn build_tutorial_wizard_defaults(&self, state: &TutorialState) -> (String, String) {
        let title = format!(
            "Tutorial - {} ({}%)",
            state.current_step.title(),
            state.progress_percent()
        );

        let mut content = TutorialSteps::content(&state.current_step);

        // Add progress bar
        let progress = state.progress_percent();
        let filled = (progress as usize) / 5; // 20 segments
        let empty = 20 - filled;
        let progress_bar = format!("\n\n[{}{}]", "█".repeat(filled), "░".repeat(empty));
        content.push_str(&progress_bar);

        // Add footer hint
        let footer = TutorialSteps::footer_hint(&state.current_step);
        content.push_str(&format!("\n\n{}", footer));

        (title, content)
    }

    fn build_profile_selector_defaults(
        &self,
        profiles: &[String],
        selected_index: usize,
    ) -> (String, String) {
        let title = "Select Profile".to_string();
        let mut content = String::from("Select a profile to switch to:\n\n");
        for (i, profile) in profiles.iter().enumerate() {
            if i == selected_index {
                content.push_str(&format!("> {} <\n", profile));
            } else {
                content.push_str(&format!("  {}\n", profile));
            }
        }
        content.push_str("\n↑/↓ to navigate, Enter to select, Esc to cancel");
        (title, content)
    }

    fn marker(selected: bool) -> &'static str {
        if selected { "> " } else { "  " }
    }

    fn masked_state<'a>(value: &str, empty_label: &'a str, set_label: &'a str) -> &'a str {
        if value.is_empty() {
            empty_label
        } else {
            set_label
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_create_profile_defaults(
        &self,
        name_input: &str,
        base_url_input: &str,
        username_input: &str,
        password_input: &str,
        api_token_input: &str,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    ) -> (String, String) {
        let title = "Create Profile".to_string();
        let mut content = String::from("Create new profile:\n\n");

        content.push_str(&format!(
            "{}Name: {}\n",
            Self::marker(selected_field == ProfileField::Name),
            name_input
        ));
        content.push_str(&format!(
            "{}Base URL: {}\n",
            Self::marker(selected_field == ProfileField::BaseUrl),
            base_url_input
        ));
        content.push_str(&format!(
            "{}Username: {}\n",
            Self::marker(selected_field == ProfileField::Username),
            username_input
        ));
        content.push_str(&format!(
            "{}Password: {}\n",
            Self::marker(selected_field == ProfileField::Password),
            Self::masked_state(password_input, "(empty)", "(set)")
        ));
        content.push_str(&format!(
            "{}API Token: {}\n",
            Self::marker(selected_field == ProfileField::ApiToken),
            Self::masked_state(api_token_input, "(empty)", "(set)")
        ));
        content.push_str(&format!(
            "{}Skip TLS Verify: {}\n",
            Self::marker(selected_field == ProfileField::SkipVerify),
            skip_verify
        ));
        content.push_str(&format!(
            "{}Timeout (s): {}\n",
            Self::marker(selected_field == ProfileField::Timeout),
            timeout_seconds
        ));
        content.push_str(&format!(
            "{}Max Retries: {}\n",
            Self::marker(selected_field == ProfileField::MaxRetries),
            max_retries
        ));
        content.push_str(&format!(
            "{}Use Keyring: {}\n",
            Self::marker(selected_field == ProfileField::UseKeyring),
            use_keyring
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_edit_profile_defaults(
        &self,
        original_name: &str,
        name_input: &str,
        base_url_input: &str,
        username_input: &str,
        password_input: &str,
        api_token_input: &str,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    ) -> (String, String) {
        let title = format!("Edit Profile '{}'", original_name);
        let mut content = String::from("Edit profile:\n\n");

        content.push_str(&format!(
            "{}Name: {}\n",
            Self::marker(selected_field == ProfileField::Name),
            name_input
        ));
        content.push_str(&format!(
            "{}Base URL: {}\n",
            Self::marker(selected_field == ProfileField::BaseUrl),
            base_url_input
        ));
        content.push_str(&format!(
            "{}Username: {}\n",
            Self::marker(selected_field == ProfileField::Username),
            username_input
        ));
        content.push_str(&format!(
            "{}Password: {}\n",
            Self::marker(selected_field == ProfileField::Password),
            Self::masked_state(password_input, "(keep existing)", "(will update)")
        ));
        content.push_str(&format!(
            "{}API Token: {}\n",
            Self::marker(selected_field == ProfileField::ApiToken),
            Self::masked_state(api_token_input, "(keep existing)", "(will update)")
        ));
        content.push_str(&format!(
            "{}Skip TLS Verify: {}\n",
            Self::marker(selected_field == ProfileField::SkipVerify),
            skip_verify
        ));
        content.push_str(&format!(
            "{}Timeout (s): {}\n",
            Self::marker(selected_field == ProfileField::Timeout),
            timeout_seconds
        ));
        content.push_str(&format!(
            "{}Max Retries: {}\n",
            Self::marker(selected_field == ProfileField::MaxRetries),
            max_retries
        ));
        content.push_str(&format!(
            "{}Use Keyring: {}\n",
            Self::marker(selected_field == ProfileField::UseKeyring),
            use_keyring
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    fn build_edit_saved_search_defaults(
        &self,
        search_name: &str,
        search_input: &str,
        description_input: &str,
        disabled: bool,
        selected_field: SavedSearchField,
    ) -> (String, String) {
        let title = format!("Edit Saved Search '{}'", search_name);
        let mut content = String::from("Edit saved search:\n\n");

        // Show name as readonly (included in navigation cycle but not editable)
        content.push_str(&format!(
            "{}Name: {} (readonly)\n",
            Self::marker(selected_field == SavedSearchField::Name),
            search_name
        ));
        content.push_str(&format!(
            "{}Search Query: {}\n",
            Self::marker(selected_field == SavedSearchField::Search),
            if search_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &search_input[..search_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            Self::marker(selected_field == SavedSearchField::Description),
            if description_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &description_input[..description_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!(
            "{}Disabled: {}\n",
            Self::marker(selected_field == SavedSearchField::Disabled),
            disabled
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    fn build_create_saved_search_defaults(
        &self,
        name_input: &str,
        search_input: &str,
        description_input: &str,
        disabled: bool,
        selected_field: SavedSearchField,
    ) -> (String, String) {
        let title = "Create Saved Search".to_string();
        let mut content = String::from("Create new saved search:\n\n");

        content.push_str(&format!(
            "{}Name: {}\n",
            Self::marker(selected_field == SavedSearchField::Name),
            if name_input.is_empty() {
                "(required)".to_string()
            } else {
                name_input.to_string()
            }
        ));
        content.push_str(&format!(
            "{}Search Query: {}\n",
            Self::marker(selected_field == SavedSearchField::Search),
            if search_input.is_empty() {
                "(required)".to_string()
            } else {
                search_input[..search_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            Self::marker(selected_field == SavedSearchField::Description),
            if description_input.is_empty() {
                "(optional)".to_string()
            } else {
                description_input[..description_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Disabled: {}\n",
            Self::marker(selected_field == SavedSearchField::Disabled),
            disabled
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_create_macro_defaults(
        &self,
        name_input: &str,
        definition_input: &str,
        args_input: &str,
        description_input: &str,
        disabled: bool,
        iseval: bool,
        selected_field: MacroField,
    ) -> (String, String) {
        let title = "Create Macro".to_string();
        let mut content = String::from("Create new macro:\n\n");

        content.push_str(&format!(
            "{}Name: {}\n",
            Self::marker(selected_field == MacroField::Name),
            name_input
        ));
        content.push_str(&format!(
            "{}Definition: {}\n",
            Self::marker(selected_field == MacroField::Definition),
            if definition_input.is_empty() {
                "(required)".to_string()
            } else {
                definition_input[..definition_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Args: {}\n",
            Self::marker(selected_field == MacroField::Args),
            if args_input.is_empty() {
                "(optional, comma-separated)".to_string()
            } else {
                args_input.to_string()
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            Self::marker(selected_field == MacroField::Description),
            if description_input.is_empty() {
                "(optional)".to_string()
            } else {
                description_input[..description_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Disabled: {}\n",
            Self::marker(selected_field == MacroField::Disabled),
            disabled
        ));
        content.push_str(&format!(
            "{}IsEval: {}\n",
            Self::marker(selected_field == MacroField::IsEval),
            iseval
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_edit_macro_defaults(
        &self,
        macro_name: &str,
        definition_input: &str,
        args_input: &str,
        description_input: &str,
        disabled: bool,
        iseval: bool,
        selected_field: MacroField,
    ) -> (String, String) {
        let title = format!("Edit Macro '{}'", macro_name);
        let mut content = String::from("Edit macro:\n\n");

        // Name is readonly
        content.push_str(&format!(
            "{}Name: {} (readonly)\n",
            Self::marker(selected_field == MacroField::Name),
            macro_name
        ));
        content.push_str(&format!(
            "{}Definition: {}\n",
            Self::marker(selected_field == MacroField::Definition),
            if definition_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &definition_input[..definition_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!(
            "{}Args: {}\n",
            Self::marker(selected_field == MacroField::Args),
            if args_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!("(will update: {})", &args_input[..args_input.len().min(40)])
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            Self::marker(selected_field == MacroField::Description),
            if description_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &description_input[..description_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!(
            "{}Disabled: {}\n",
            Self::marker(selected_field == MacroField::Disabled),
            disabled
        ));
        content.push_str(&format!(
            "{}IsEval: {}\n",
            Self::marker(selected_field == MacroField::IsEval),
            iseval
        ));

        content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
        (title, content)
    }

    fn build_command_palette_defaults(
        &self,
        input: &str,
        selected_index: usize,
        items: &[crate::app::command_palette::CommandPaletteItem],
    ) -> (String, String) {
        let title = "Command Palette".to_string();
        let mut content = String::new();

        // Search prompt
        content.push_str("> ");
        if input.is_empty() {
            content.push_str("Type to search commands...");
        } else {
            content.push_str(input);
        }
        content.push('\n');
        content.push_str(&"─".repeat(50));
        content.push('\n');

        if items.is_empty() {
            content.push_str("No matching commands\n");
        } else {
            for (i, item) in items.iter().enumerate().take(15) {
                let marker = if i == selected_index { "> " } else { "  " };
                let recent_marker = if item.is_recent { "★ " } else { "  " };

                content.push_str(marker);
                content.push_str(recent_marker);
                content.push_str(&item.name);

                if let Some(ref shortcut) = item.shortcut {
                    content.push_str(&format!(" [{}]", shortcut));
                }

                content.push('\n');
            }

            if items.len() > 15 {
                content.push_str(&format!("\n... and {} more\n", items.len() - 15));
            }
        }

        content.push('\n');
        content.push_str("↑/↓ or j/k: Navigate  Enter: Execute  Esc: Close");

        (title, content)
    }

    fn build_auth_recovery_defaults(&self, kind: &AuthRecoveryKind) -> (String, String) {
        let title = "Authentication Error".to_string();

        let (diagnosis, next_steps) = match kind {
            AuthRecoveryKind::InvalidCredentials => (
                "Invalid credentials detected. The username, password, or API token is incorrect.",
                "• Check your username and password\n• Verify your API token is valid\n• Ensure your account has not been locked",
            ),
            AuthRecoveryKind::SessionExpired => (
                "Your session has expired due to inactivity.",
                "• Re-authenticate with your credentials\n• Consider using an API token for persistent sessions",
            ),
            AuthRecoveryKind::MissingAuthConfig => (
                "No authentication configuration found.",
                "• Create a new profile with 'n'\n• Select an existing profile with 'p'\n• Set SPLUNK_USERNAME and SPLUNK_PASSWORD environment variables",
            ),
            AuthRecoveryKind::TlsOrCertificate => (
                "TLS certificate verification failed.",
                "• Verify the server's TLS certificate is valid\n• Check system time is correct\n• Consider enabling 'Skip TLS Verify' for self-signed certificates (development only)",
            ),
            AuthRecoveryKind::ConnectionRefused => (
                "Unable to connect to the Splunk server.",
                "• Verify the Splunk server is running\n• Check the base URL in your profile\n• Ensure network connectivity to the server",
            ),
            AuthRecoveryKind::Timeout => (
                "Connection timed out while authenticating.",
                "• Check network connectivity\n• Verify the Splunk server is responsive\n• Consider increasing the timeout setting in your profile",
            ),
            AuthRecoveryKind::Unknown => (
                "An unknown authentication error occurred.",
                "• Check the error details for more information\n• Verify your profile configuration\n• Try re-authenticating",
            ),
        };

        let content = format!(
            "{diagnosis}

Next Steps:
{next_steps}

───

Keybindings:
  r  Retry authentication
  p  Open profile selector
  n  Create new profile
  e  View error details
  Esc  Close this panel

───

Configuration:
  Set SPLUNK_CONFIG_PATH env var to specify a custom config file location."
        );

        (title, content)
    }

    fn build_connection_diagnostics_defaults(
        &self,
        result: &ConnectionDiagnosticsResult,
    ) -> (String, String) {
        let title = "Connection Diagnostics".to_string();
        let mut content = String::new();

        fn status_icon(status: DiagnosticStatus) -> &'static str {
            match status {
                DiagnosticStatus::Pass => "✓",
                DiagnosticStatus::Fail => "✗",
                DiagnosticStatus::Skip => "○",
            }
        }

        fn status_text(status: DiagnosticStatus) -> &'static str {
            match status {
                DiagnosticStatus::Pass => "PASS",
                DiagnosticStatus::Fail => "FAIL",
                DiagnosticStatus::Skip => "SKIP",
            }
        }

        // Reachability
        content.push_str(&format!(
            "{} {}",
            status_icon(result.reachable.status),
            result.reachable.name
        ));
        if result.reachable.status != DiagnosticStatus::Skip {
            content.push_str(&format!(" ({}ms)", result.reachable.duration_ms));
        }
        if let Some(ref err) = result.reachable.error {
            content.push_str(&format!(": {}", err));
        }
        content.push('\n');

        // Authentication
        content.push_str(&format!(
            "{} {}",
            status_icon(result.auth.status),
            result.auth.name
        ));
        if let Some(ref err) = result.auth.error {
            content.push_str(&format!(": {}", err));
        }
        content.push('\n');

        // TLS
        content.push_str(&format!(
            "{} {}",
            status_icon(result.tls.status),
            result.tls.name
        ));
        if let Some(ref err) = result.tls.error {
            content.push_str(&format!(": {}", err));
        }
        content.push('\n');

        // Server info
        if let Some(ref info) = result.server_info {
            content.push_str(&format!(
                "\nServer: {} (v{}, {})\n",
                info.server_name, info.version, info.build
            ));
            if let Some(ref mode) = info.mode {
                content.push_str(&format!("Mode: {}\n", mode));
            }
        }

        content.push_str(&format!(
            "\nStatus: {} {}\n",
            status_icon(result.overall_status),
            status_text(result.overall_status)
        ));

        // Remediation hints
        if !result.remediation_hints.is_empty() {
            content.push_str("\nRemediation:\n");
            for hint in &result.remediation_hints {
                content.push_str(&format!("• {}\n", hint));
            }
        }

        content.push_str("\nPress Enter to close");

        (title, content)
    }
}
