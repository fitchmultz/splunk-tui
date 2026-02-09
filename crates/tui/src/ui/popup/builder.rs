//! Popup struct and builder implementation.
//!
//! This module provides the `Popup` struct and `PopupBuilder` for constructing
//! popup dialogs with customizable titles and content.

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

    fn build_defaults(&self) -> (String, String) {
        match &self.kind {
            PopupType::Help => ("Help".to_string(), help::help_text()),
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

        let name_marker = if selected_field == ProfileField::Name {
            "> "
        } else {
            "  "
        };
        let base_url_marker = if selected_field == ProfileField::BaseUrl {
            "> "
        } else {
            "  "
        };
        let username_marker = if selected_field == ProfileField::Username {
            "> "
        } else {
            "  "
        };
        let password_marker = if selected_field == ProfileField::Password {
            "> "
        } else {
            "  "
        };
        let api_token_marker = if selected_field == ProfileField::ApiToken {
            "> "
        } else {
            "  "
        };
        let skip_verify_marker = if selected_field == ProfileField::SkipVerify {
            "> "
        } else {
            "  "
        };
        let timeout_marker = if selected_field == ProfileField::Timeout {
            "> "
        } else {
            "  "
        };
        let max_retries_marker = if selected_field == ProfileField::MaxRetries {
            "> "
        } else {
            "  "
        };
        let use_keyring_marker = if selected_field == ProfileField::UseKeyring {
            "> "
        } else {
            "  "
        };

        content.push_str(&format!("{}Name: {}\n", name_marker, name_input));
        content.push_str(&format!(
            "{}Base URL: {}\n",
            base_url_marker, base_url_input
        ));
        content.push_str(&format!(
            "{}Username: {}\n",
            username_marker, username_input
        ));
        let password_display = if password_input.is_empty() {
            "(empty)".to_string()
        } else {
            "(set)".to_string()
        };
        content.push_str(&format!(
            "{}Password: {}\n",
            password_marker, password_display
        ));
        let token_display = if api_token_input.is_empty() {
            "(empty)".to_string()
        } else {
            "(set)".to_string()
        };
        content.push_str(&format!(
            "{}API Token: {}\n",
            api_token_marker, token_display
        ));
        content.push_str(&format!(
            "{}Skip TLS Verify: {}\n",
            skip_verify_marker, skip_verify
        ));
        content.push_str(&format!(
            "{}Timeout (s): {}\n",
            timeout_marker, timeout_seconds
        ));
        content.push_str(&format!(
            "{}Max Retries: {}\n",
            max_retries_marker, max_retries
        ));
        content.push_str(&format!(
            "{}Use Keyring: {}\n",
            use_keyring_marker, use_keyring
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

        let name_marker = if selected_field == ProfileField::Name {
            "> "
        } else {
            "  "
        };
        let base_url_marker = if selected_field == ProfileField::BaseUrl {
            "> "
        } else {
            "  "
        };
        let username_marker = if selected_field == ProfileField::Username {
            "> "
        } else {
            "  "
        };
        let password_marker = if selected_field == ProfileField::Password {
            "> "
        } else {
            "  "
        };
        let api_token_marker = if selected_field == ProfileField::ApiToken {
            "> "
        } else {
            "  "
        };
        let skip_verify_marker = if selected_field == ProfileField::SkipVerify {
            "> "
        } else {
            "  "
        };
        let timeout_marker = if selected_field == ProfileField::Timeout {
            "> "
        } else {
            "  "
        };
        let max_retries_marker = if selected_field == ProfileField::MaxRetries {
            "> "
        } else {
            "  "
        };
        let use_keyring_marker = if selected_field == ProfileField::UseKeyring {
            "> "
        } else {
            "  "
        };

        content.push_str(&format!("{}Name: {}\n", name_marker, name_input));
        content.push_str(&format!(
            "{}Base URL: {}\n",
            base_url_marker, base_url_input
        ));
        content.push_str(&format!(
            "{}Username: {}\n",
            username_marker, username_input
        ));
        let password_display = if password_input.is_empty() {
            "(keep existing)".to_string()
        } else {
            "(will update)".to_string()
        };
        content.push_str(&format!(
            "{}Password: {}\n",
            password_marker, password_display
        ));
        let token_display = if api_token_input.is_empty() {
            "(keep existing)".to_string()
        } else {
            "(will update)".to_string()
        };
        content.push_str(&format!(
            "{}API Token: {}\n",
            api_token_marker, token_display
        ));
        content.push_str(&format!(
            "{}Skip TLS Verify: {}\n",
            skip_verify_marker, skip_verify
        ));
        content.push_str(&format!(
            "{}Timeout (s): {}\n",
            timeout_marker, timeout_seconds
        ));
        content.push_str(&format!(
            "{}Max Retries: {}\n",
            max_retries_marker, max_retries
        ));
        content.push_str(&format!(
            "{}Use Keyring: {}\n",
            use_keyring_marker, use_keyring
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

        let name_marker = if selected_field == SavedSearchField::Name {
            "> "
        } else {
            "  "
        };
        let search_marker = if selected_field == SavedSearchField::Search {
            "> "
        } else {
            "  "
        };
        let description_marker = if selected_field == SavedSearchField::Description {
            "> "
        } else {
            "  "
        };
        let disabled_marker = if selected_field == SavedSearchField::Disabled {
            "> "
        } else {
            "  "
        };

        // Show name as readonly (included in navigation cycle but not editable)
        content.push_str(&format!(
            "{}Name: {} (readonly)\n",
            name_marker, search_name
        ));
        content.push_str(&format!(
            "{}Search Query: {}\n",
            search_marker,
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
            description_marker,
            if description_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &description_input[..description_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!("{}Disabled: {}\n", disabled_marker, disabled));

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

        let name_marker = if selected_field == SavedSearchField::Name {
            "> "
        } else {
            "  "
        };
        let search_marker = if selected_field == SavedSearchField::Search {
            "> "
        } else {
            "  "
        };
        let description_marker = if selected_field == SavedSearchField::Description {
            "> "
        } else {
            "  "
        };
        let disabled_marker = if selected_field == SavedSearchField::Disabled {
            "> "
        } else {
            "  "
        };

        content.push_str(&format!(
            "{}Name: {}\n",
            name_marker,
            if name_input.is_empty() {
                "(required)".to_string()
            } else {
                name_input.to_string()
            }
        ));
        content.push_str(&format!(
            "{}Search Query: {}\n",
            search_marker,
            if search_input.is_empty() {
                "(required)".to_string()
            } else {
                search_input[..search_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            description_marker,
            if description_input.is_empty() {
                "(optional)".to_string()
            } else {
                description_input[..description_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!("{}Disabled: {}\n", disabled_marker, disabled));

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

        let name_marker = if selected_field == MacroField::Name {
            "> "
        } else {
            "  "
        };
        let definition_marker = if selected_field == MacroField::Definition {
            "> "
        } else {
            "  "
        };
        let args_marker = if selected_field == MacroField::Args {
            "> "
        } else {
            "  "
        };
        let description_marker = if selected_field == MacroField::Description {
            "> "
        } else {
            "  "
        };
        let disabled_marker = if selected_field == MacroField::Disabled {
            "> "
        } else {
            "  "
        };
        let iseval_marker = if selected_field == MacroField::IsEval {
            "> "
        } else {
            "  "
        };

        content.push_str(&format!("{}Name: {}\n", name_marker, name_input));
        content.push_str(&format!(
            "{}Definition: {}\n",
            definition_marker,
            if definition_input.is_empty() {
                "(required)".to_string()
            } else {
                definition_input[..definition_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!(
            "{}Args: {}\n",
            args_marker,
            if args_input.is_empty() {
                "(optional, comma-separated)".to_string()
            } else {
                args_input.to_string()
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            description_marker,
            if description_input.is_empty() {
                "(optional)".to_string()
            } else {
                description_input[..description_input.len().min(40)].to_string()
            }
        ));
        content.push_str(&format!("{}Disabled: {}\n", disabled_marker, disabled));
        content.push_str(&format!("{}IsEval: {}\n", iseval_marker, iseval));

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

        let name_marker = if selected_field == MacroField::Name {
            "> "
        } else {
            "  "
        };
        let definition_marker = if selected_field == MacroField::Definition {
            "> "
        } else {
            "  "
        };
        let args_marker = if selected_field == MacroField::Args {
            "> "
        } else {
            "  "
        };
        let description_marker = if selected_field == MacroField::Description {
            "> "
        } else {
            "  "
        };
        let disabled_marker = if selected_field == MacroField::Disabled {
            "> "
        } else {
            "  "
        };
        let iseval_marker = if selected_field == MacroField::IsEval {
            "> "
        } else {
            "  "
        };

        // Name is readonly
        content.push_str(&format!("{}Name: {} (readonly)\n", name_marker, macro_name));
        content.push_str(&format!(
            "{}Definition: {}\n",
            definition_marker,
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
            args_marker,
            if args_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!("(will update: {})", &args_input[..args_input.len().min(40)])
            }
        ));
        content.push_str(&format!(
            "{}Description: {}\n",
            description_marker,
            if description_input.is_empty() {
                "(unchanged)".to_string()
            } else {
                format!(
                    "(will update: {})",
                    &description_input[..description_input.len().min(40)]
                )
            }
        ));
        content.push_str(&format!("{}Disabled: {}\n", disabled_marker, disabled));
        content.push_str(&format!("{}IsEval: {}\n", iseval_marker, iseval));

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
}
