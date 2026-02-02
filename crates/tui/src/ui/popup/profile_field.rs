//! Profile field selection for form navigation.
//!
//! This module provides the `ProfileField` enum and its navigation methods
//! for cycling through profile form fields.

/// Field selection for profile form navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileField {
    /// Profile name field
    Name,
    /// Base URL field
    BaseUrl,
    /// Username field
    Username,
    /// Password field
    Password,
    /// API token field
    ApiToken,
    /// Skip TLS verification toggle
    SkipVerify,
    /// Timeout seconds field
    Timeout,
    /// Max retries field
    MaxRetries,
    /// Use keyring toggle
    UseKeyring,
}

impl ProfileField {
    /// Get the next field in the form (cycles through all fields).
    pub fn next(self) -> Self {
        match self {
            ProfileField::Name => ProfileField::BaseUrl,
            ProfileField::BaseUrl => ProfileField::Username,
            ProfileField::Username => ProfileField::Password,
            ProfileField::Password => ProfileField::ApiToken,
            ProfileField::ApiToken => ProfileField::SkipVerify,
            ProfileField::SkipVerify => ProfileField::Timeout,
            ProfileField::Timeout => ProfileField::MaxRetries,
            ProfileField::MaxRetries => ProfileField::UseKeyring,
            ProfileField::UseKeyring => ProfileField::Name,
        }
    }

    /// Get the previous field in the form (cycles through all fields).
    pub fn previous(self) -> Self {
        match self {
            ProfileField::Name => ProfileField::UseKeyring,
            ProfileField::BaseUrl => ProfileField::Name,
            ProfileField::Username => ProfileField::BaseUrl,
            ProfileField::Password => ProfileField::Username,
            ProfileField::ApiToken => ProfileField::Password,
            ProfileField::SkipVerify => ProfileField::ApiToken,
            ProfileField::Timeout => ProfileField::SkipVerify,
            ProfileField::MaxRetries => ProfileField::Timeout,
            ProfileField::UseKeyring => ProfileField::MaxRetries,
        }
    }
}
