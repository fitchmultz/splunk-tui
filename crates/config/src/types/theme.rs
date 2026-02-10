//! Theme types for Splunk TUI configuration.
//!
//! Responsibilities:
//! - Define user-selectable color themes (`ColorTheme`).
//! - Define the expanded runtime `Theme` with all color values.
//! - Provide conversion from `ColorTheme` to `Theme`.
//!
//! Does NOT handle:
//! - Actual rendering (see TUI crate).
//! - Theme persistence (see `persistence` module which persists `ColorTheme`).
//!
//! Invariants:
//! - `ColorTheme` is the persisted representation; `Theme` is the runtime representation.
//! - `Theme` is intentionally NOT serializable - always persist `ColorTheme`.
//! - Colors are semantically named (error/warn/success/info) for consistent usage.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fmt;

/// User-selectable color theme.
///
/// This is persisted to disk via `PersistedState` and expanded into a full `Theme` at runtime.
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

/// Expanded runtime theme.
///
/// Invariants:
/// - This is intentionally **not serialized**. Persist `ColorTheme` and expand on startup.
/// - Colors should be semantically meaningful (error/warn/success/info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // Global / chrome
    pub background: Color,
    pub text: Color,
    pub text_dim: Color,
    pub border: Color,
    pub title: Color,
    pub accent: Color,

    // Selection / highlight
    pub highlight_fg: Color,
    pub highlight_bg: Color,

    // Semantics
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub disabled: Color,

    // Tables
    pub table_header_fg: Color,
    pub table_header_bg: Color,

    // Health indicator
    pub health_healthy: Color,
    pub health_unhealthy: Color,
    pub health_unknown: Color,

    // Logs
    pub log_error: Color,
    pub log_warn: Color,
    pub log_info: Color,
    pub log_debug: Color,
    pub log_component: Color,

    // Syntax highlighting
    pub syntax_command: Color,
    pub syntax_operator: Color,
    pub syntax_function: Color,
    pub syntax_string: Color,
    pub syntax_number: Color,
    pub syntax_comment: Color,
    pub syntax_punctuation: Color,
    pub syntax_pipe: Color,
    pub syntax_comparison: Color,
}

impl Theme {
    /// Expand a persisted `ColorTheme` into a full runtime palette.
    pub fn from_color_theme(theme: ColorTheme) -> Self {
        match theme {
            ColorTheme::Default => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::Cyan,
                title: Color::Cyan,
                accent: Color::Yellow,

                highlight_fg: Color::Yellow,
                highlight_bg: Color::DarkGray,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                disabled: Color::DarkGray,

                table_header_fg: Color::Cyan,
                table_header_bg: Color::DarkGray,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Blue,
                log_component: Color::Magenta,

                syntax_command: Color::Cyan,
                syntax_operator: Color::Magenta,
                syntax_function: Color::Blue,
                syntax_string: Color::Green,
                syntax_number: Color::Blue,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::DarkGray,
                syntax_pipe: Color::Yellow,
                syntax_comparison: Color::Red,
            },
            ColorTheme::Light => Self {
                background: Color::White,
                text: Color::Black,
                text_dim: Color::Gray,
                border: Color::Blue,
                title: Color::Blue,
                accent: Color::Magenta,

                highlight_fg: Color::Black,
                highlight_bg: Color::Gray,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                disabled: Color::Gray,

                table_header_fg: Color::Black,
                table_header_bg: Color::Gray,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Blue,
                log_component: Color::Magenta,

                syntax_command: Color::Blue,
                syntax_operator: Color::Magenta,
                syntax_function: Color::Blue,
                syntax_string: Color::Green,
                syntax_number: Color::Blue,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::Gray,
                syntax_pipe: Color::Magenta,
                syntax_comparison: Color::Red,
            },
            ColorTheme::Dark => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::Indexed(110), // soft blue/cyan
                title: Color::Indexed(110),
                accent: Color::Indexed(214), // orange-ish

                highlight_fg: Color::White,
                highlight_bg: Color::Indexed(236),

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Indexed(110),
                disabled: Color::DarkGray,

                table_header_fg: Color::Indexed(110),
                table_header_bg: Color::Indexed(236),

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Indexed(110),
                log_component: Color::Indexed(176),

                syntax_command: Color::Indexed(110),
                syntax_operator: Color::Indexed(176),
                syntax_function: Color::Indexed(75),
                syntax_string: Color::Green,
                syntax_number: Color::Indexed(75),
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::DarkGray,
                syntax_pipe: Color::Indexed(214),
                syntax_comparison: Color::Red,
            },
            ColorTheme::HighContrast => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::White,
                title: Color::White,
                accent: Color::Yellow,

                highlight_fg: Color::White,
                highlight_bg: Color::Blue,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                disabled: Color::Gray,

                table_header_fg: Color::Black,
                table_header_bg: Color::White,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Cyan,
                log_component: Color::Yellow,

                syntax_command: Color::Cyan,
                syntax_operator: Color::Yellow,
                syntax_function: Color::Magenta,
                syntax_string: Color::Green,
                syntax_number: Color::Cyan,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::White,
                syntax_pipe: Color::Yellow,
                syntax_comparison: Color::Red,
            },
            ColorTheme::Deuteranopia => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Indexed(250),
                border: Color::Indexed(75), // Blue
                title: Color::Indexed(75),
                accent: Color::Indexed(226), // Yellow
                highlight_fg: Color::Black,
                highlight_bg: Color::Indexed(226), // Yellow
                success: Color::Indexed(33),       // Blue (instead of green)
                warning: Color::Indexed(226),      // Yellow
                error: Color::Indexed(208),        // Orange-red
                info: Color::Indexed(75),          // Blue
                disabled: Color::Indexed(242),
                table_header_fg: Color::Black,
                table_header_bg: Color::Indexed(226),
                health_healthy: Color::Indexed(33),
                health_unhealthy: Color::Indexed(208),
                health_unknown: Color::Indexed(226),
                log_error: Color::Indexed(208),
                log_warn: Color::Indexed(226),
                log_info: Color::Indexed(33),
                log_debug: Color::Indexed(75),
                log_component: Color::Indexed(189),
                syntax_command: Color::Indexed(75),
                syntax_operator: Color::Indexed(226),
                syntax_function: Color::Indexed(33),
                syntax_string: Color::Indexed(189),
                syntax_number: Color::Indexed(75),
                syntax_comment: Color::Indexed(242),
                syntax_punctuation: Color::Indexed(250),
                syntax_pipe: Color::Indexed(226),
                syntax_comparison: Color::Indexed(208),
            },
            ColorTheme::Protanopia => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Indexed(250),
                border: Color::Indexed(39), // Bright blue
                title: Color::Indexed(39),
                accent: Color::Indexed(214), // Orange
                highlight_fg: Color::Black,
                highlight_bg: Color::Indexed(214), // Orange
                success: Color::Indexed(39),       // Blue
                warning: Color::Indexed(214),      // Orange
                error: Color::Indexed(202),        // Deep orange/red
                info: Color::Indexed(39),
                disabled: Color::Indexed(242),
                table_header_fg: Color::Black,
                table_header_bg: Color::Indexed(214),
                health_healthy: Color::Indexed(39),
                health_unhealthy: Color::Indexed(202),
                health_unknown: Color::Indexed(214),
                log_error: Color::Indexed(202),
                log_warn: Color::Indexed(214),
                log_info: Color::Indexed(39),
                log_debug: Color::Indexed(75),
                log_component: Color::Indexed(189),
                syntax_command: Color::Indexed(39),
                syntax_operator: Color::Indexed(214),
                syntax_function: Color::Indexed(33),
                syntax_string: Color::Indexed(189),
                syntax_number: Color::Indexed(39),
                syntax_comment: Color::Indexed(242),
                syntax_punctuation: Color::Indexed(250),
                syntax_pipe: Color::Indexed(214),
                syntax_comparison: Color::Indexed(202),
            },
            ColorTheme::Tritanopia => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Indexed(250),
                border: Color::Indexed(197), // Pink-red
                title: Color::Indexed(197),
                accent: Color::Indexed(37), // Teal/cyan
                highlight_fg: Color::Black,
                highlight_bg: Color::Indexed(37), // Teal
                success: Color::Indexed(37),      // Teal
                warning: Color::Indexed(197),     // Pink-red
                error: Color::Indexed(161),       // Magenta/deep pink
                info: Color::Indexed(37),
                disabled: Color::Indexed(242),
                table_header_fg: Color::Black,
                table_header_bg: Color::Indexed(37),
                health_healthy: Color::Indexed(37),
                health_unhealthy: Color::Indexed(161),
                health_unknown: Color::Indexed(197),
                log_error: Color::Indexed(161),
                log_warn: Color::Indexed(197),
                log_info: Color::Indexed(37),
                log_debug: Color::Indexed(44),
                log_component: Color::Indexed(213),
                syntax_command: Color::Indexed(37),
                syntax_operator: Color::Indexed(197),
                syntax_function: Color::Indexed(44),
                syntax_string: Color::Indexed(213),
                syntax_number: Color::Indexed(37),
                syntax_comment: Color::Indexed(242),
                syntax_punctuation: Color::Indexed(250),
                syntax_pipe: Color::Indexed(197),
                syntax_comparison: Color::Indexed(161),
            },
            ColorTheme::Monochrome => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Indexed(245),
                border: Color::Indexed(250),
                title: Color::White,
                accent: Color::Indexed(255),
                highlight_fg: Color::Black,
                highlight_bg: Color::White,
                success: Color::Indexed(250), // Very light
                warning: Color::Indexed(245), // Light
                error: Color::Indexed(255),   // Brightest
                info: Color::Indexed(245),
                disabled: Color::Indexed(238),
                table_header_fg: Color::Black,
                table_header_bg: Color::White,
                health_healthy: Color::Indexed(250),
                health_unhealthy: Color::Indexed(255),
                health_unknown: Color::Indexed(245),
                log_error: Color::Indexed(255),
                log_warn: Color::Indexed(245),
                log_info: Color::Indexed(250),
                log_debug: Color::Indexed(238),
                log_component: Color::Indexed(245),
                syntax_command: Color::White,
                syntax_operator: Color::Indexed(250),
                syntax_function: Color::Indexed(245),
                syntax_string: Color::Indexed(250),
                syntax_number: Color::Indexed(245),
                syntax_comment: Color::Indexed(238),
                syntax_punctuation: Color::Indexed(245),
                syntax_pipe: Color::Indexed(250),
                syntax_comparison: Color::Indexed(255),
            },
        }
    }
}

impl From<ColorTheme> for Theme {
    fn from(value: ColorTheme) -> Self {
        Self::from_color_theme(value)
    }
}

impl Default for Theme {
    fn default() -> Self {
        ColorTheme::Default.into()
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
        assert!(matches!(
            ColorTheme::Default.cycle_next(),
            ColorTheme::Light
        ));
        assert!(matches!(ColorTheme::Light.cycle_next(), ColorTheme::Dark));
        assert!(matches!(
            ColorTheme::Dark.cycle_next(),
            ColorTheme::HighContrast
        ));
        assert!(matches!(
            ColorTheme::HighContrast.cycle_next(),
            ColorTheme::Deuteranopia
        ));
        assert!(matches!(
            ColorTheme::Deuteranopia.cycle_next(),
            ColorTheme::Protanopia
        ));
        assert!(matches!(
            ColorTheme::Protanopia.cycle_next(),
            ColorTheme::Tritanopia
        ));
        assert!(matches!(
            ColorTheme::Tritanopia.cycle_next(),
            ColorTheme::Monochrome
        ));
        assert!(matches!(
            ColorTheme::Monochrome.cycle_next(),
            ColorTheme::Default
        ));
    }

    #[test]
    fn test_theme_from_color_theme() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        assert_eq!(theme.background, Color::Black);
        assert_eq!(theme.text, Color::White);
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme.background, Color::Black);
        assert_eq!(theme.text, Color::White);
    }

    #[test]
    fn test_color_theme_serde_round_trip() {
        let original = ColorTheme::Dark;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ColorTheme = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_color_theme_serde_all_variants() {
        // Verify all theme variants serialize and deserialize correctly
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
            assert_eq!(theme, deserialized, "Serde failed for {:?}", theme);
        }
    }

    #[test]
    fn test_color_theme_serde_snake_case() {
        // Verify snake_case serialization
        assert_eq!(
            serde_json::to_string(&ColorTheme::HighContrast).unwrap(),
            "\"high_contrast\""
        );
        assert_eq!(
            serde_json::to_string(&ColorTheme::Deuteranopia).unwrap(),
            "\"deuteranopia\""
        );
        assert_eq!(
            serde_json::to_string(&ColorTheme::Protanopia).unwrap(),
            "\"protanopia\""
        );
        assert_eq!(
            serde_json::to_string(&ColorTheme::Tritanopia).unwrap(),
            "\"tritanopia\""
        );
        assert_eq!(
            serde_json::to_string(&ColorTheme::Monochrome).unwrap(),
            "\"monochrome\""
        );
    }

    #[test]
    fn test_color_theme_display() {
        assert_eq!(format!("{}", ColorTheme::Default), "Default");
        assert_eq!(format!("{}", ColorTheme::Light), "Light");
    }

    #[test]
    fn test_all_themes_convertible() {
        // Verify all themes can be converted to runtime Theme
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
            let runtime_theme = Theme::from_color_theme(theme);
            // Basic sanity checks
            assert_ne!(runtime_theme.background, runtime_theme.text);
            assert_ne!(runtime_theme.success, runtime_theme.error);
        }
    }
}
