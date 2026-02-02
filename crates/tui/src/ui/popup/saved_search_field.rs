//! Saved search field selection for form navigation.
//!
//! This module provides the `SavedSearchField` enum and its navigation methods
//! for cycling through saved search form fields.

/// Field selection for saved search form navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedSearchField {
    /// Search query field (SPL)
    Search,
    /// Description field
    Description,
    /// Disabled toggle field
    Disabled,
}

impl SavedSearchField {
    /// Get the next field in the form (cycles through all fields).
    pub fn next(self) -> Self {
        match self {
            SavedSearchField::Search => SavedSearchField::Description,
            SavedSearchField::Description => SavedSearchField::Disabled,
            SavedSearchField::Disabled => SavedSearchField::Search,
        }
    }

    /// Get the previous field in the form (cycles through all fields).
    pub fn previous(self) -> Self {
        match self {
            SavedSearchField::Search => SavedSearchField::Disabled,
            SavedSearchField::Description => SavedSearchField::Search,
            SavedSearchField::Disabled => SavedSearchField::Description,
        }
    }
}
