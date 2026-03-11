//! Persisted theme selection types for Splunk TUI configuration.
//!
//! Responsibilities:
//! - Define user-selectable color themes (`ColorTheme`).
//! - Provide display helpers and cycle order for persisted theme selection.
//!
//! Does NOT handle:
//! - Runtime theme expansion (see `splunk-tui::theme`).
//! - Rendering or widget styling.
//!
//! Invariants:
//! - `ColorTheme` is the persisted representation.
//! - Runtime color values live in the TUI crate.

use serde::{Deserialize, Serialize};
use std::fmt;

/// User-selectable color theme persisted in config/state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorTheme {
    #[default]
    Default,
    Light,
    Dark,
    HighContrast,
    Deuteranopia,
    Protanopia,
    Tritanopia,
    Monochrome,
}

impl ColorTheme {
    /// Human-readable display name for UI surfaces.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::HighContrast => "High Contrast",
            Self::Deuteranopia => "Deuteranopia (Blue/Yellow)",
            Self::Protanopia => "Protanopia (Blue/Orange)",
            Self::Tritanopia => "Tritanopia (Red/Teal)",
            Self::Monochrome => "Monochrome",
        }
    }

    /// Next theme in the cycle (used by Settings screen "t" key).
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Default => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::HighContrast,
            Self::HighContrast => Self::Deuteranopia,
            Self::Deuteranopia => Self::Protanopia,
            Self::Protanopia => Self::Tritanopia,
            Self::Tritanopia => Self::Monochrome,
            Self::Monochrome => Self::Default,
        }
    }
}

impl fmt::Display for ColorTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_theme_display_name() {
        assert_eq!(ColorTheme::Default.display_name(), "Default");
        assert_eq!(ColorTheme::Light.display_name(), "Light");
        assert_eq!(ColorTheme::Dark.display_name(), "Dark");
        assert_eq!(ColorTheme::HighContrast.display_name(), "High Contrast");
        assert_eq!(
            ColorTheme::Deuteranopia.display_name(),
            "Deuteranopia (Blue/Yellow)"
        );
        assert_eq!(
            ColorTheme::Protanopia.display_name(),
            "Protanopia (Blue/Orange)"
        );
        assert_eq!(
            ColorTheme::Tritanopia.display_name(),
            "Tritanopia (Red/Teal)"
        );
        assert_eq!(ColorTheme::Monochrome.display_name(), "Monochrome");
    }

    #[test]
    fn test_color_theme_cycle_next() {
        assert_eq!(ColorTheme::Default.cycle_next(), ColorTheme::Light);
        assert!(matches!(ColorTheme::Light.cycle_next(), ColorTheme::Dark));
        assert_eq!(ColorTheme::Dark.cycle_next(), ColorTheme::HighContrast);
        assert_eq!(
            ColorTheme::HighContrast.cycle_next(),
            ColorTheme::Deuteranopia
        );
        assert_eq!(
            ColorTheme::Deuteranopia.cycle_next(),
            ColorTheme::Protanopia
        );
        assert_eq!(ColorTheme::Protanopia.cycle_next(), ColorTheme::Tritanopia);
        assert_eq!(ColorTheme::Tritanopia.cycle_next(), ColorTheme::Monochrome);
        assert_eq!(ColorTheme::Monochrome.cycle_next(), ColorTheme::Default);
    }

    #[test]
    fn test_color_theme_serialization() {
        let original = ColorTheme::Dark;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ColorTheme = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_color_theme_variants_round_trip() {
        for theme in [
            ColorTheme::Default,
            ColorTheme::Light,
            ColorTheme::Dark,
            ColorTheme::HighContrast,
            ColorTheme::Deuteranopia,
            ColorTheme::Protanopia,
            ColorTheme::Tritanopia,
            ColorTheme::Monochrome,
        ] {
            let json = serde_json::to_string(&theme).unwrap();
            let deserialized: ColorTheme = serde_json::from_str(&json).unwrap();
            assert_eq!(theme, deserialized);
        }
    }
}
