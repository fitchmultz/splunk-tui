//! TUI-specific theme helpers and style builders.
//!
//! This module extends `splunk_config::Theme` with ergonomic helpers
//! for building ratatui `Style` objects consistently across the TUI.

use ratatui::style::{Modifier, Style};
use splunk_config::Theme;

/// Spinner characters for animated loading indicator.
///
/// These Braille patterns create a smooth spinning animation when cycled.
pub const SPINNER_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

/// Get the spinner character for a given animation frame.
///
/// This helper handles the modulo operation to cycle through the spinner characters.
///
/// # Arguments
///
/// * `frame` - The current animation frame (typically increments each render cycle)
///
/// # Returns
///
/// The spinner character to display for this frame.
///
/// # Example
///
/// ```
/// use splunk_tui::ui::theme::spinner_char;
///
/// let char_for_frame = spinner_char(5);
/// ```
pub fn spinner_char(frame: u8) -> char {
    SPINNER_CHARS[frame as usize % SPINNER_CHARS.len()]
}

/// Trait extending Theme with helper methods for creating styled widgets.
pub trait ThemeExt {
    /// Get the base text style.
    fn text(&self) -> Style;
    /// Get dimmed text style.
    fn text_dim(&self) -> Style;
    /// Get title style (accent + bold).
    fn title(&self) -> Style;
    /// Get border style.
    fn border(&self) -> Style;
    /// Get border style when focused.
    fn border_focused(&self) -> Style;
    /// Get highlight/selection style.
    fn highlight(&self) -> Style;
    /// Get success style.
    fn success(&self) -> Style;
    /// Get warning style.
    fn warning(&self) -> Style;
    /// Get error style.
    fn error(&self) -> Style;
    /// Get info style.
    fn info(&self) -> Style;
    /// Get disabled style.
    fn disabled(&self) -> Style;
    /// Get table header style.
    fn table_header(&self) -> Style;
    /// Get syntax styles.
    fn syntax_command(&self) -> Style;
    fn syntax_string(&self) -> Style;
    fn syntax_number(&self) -> Style;
    fn syntax_comment(&self) -> Style;
}

impl ThemeExt for Theme {
    fn text(&self) -> Style {
        Style::default().fg(self.text)
    }

    fn text_dim(&self) -> Style {
        Style::default().fg(self.text_dim)
    }

    fn title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    fn border_focused(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    fn highlight(&self) -> Style {
        Style::default().fg(self.highlight_fg).bg(self.highlight_bg)
    }

    fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    fn info(&self) -> Style {
        Style::default().fg(self.info)
    }

    fn disabled(&self) -> Style {
        Style::default().fg(self.disabled)
    }

    fn table_header(&self) -> Style {
        Style::default()
            .fg(self.table_header_fg)
            .bg(self.table_header_bg)
            .add_modifier(Modifier::BOLD)
    }

    fn syntax_command(&self) -> Style {
        Style::default()
            .fg(self.syntax_command)
            .add_modifier(Modifier::BOLD)
    }

    fn syntax_string(&self) -> Style {
        Style::default().fg(self.syntax_string)
    }

    fn syntax_number(&self) -> Style {
        Style::default().fg(self.syntax_number)
    }

    fn syntax_comment(&self) -> Style {
        Style::default().fg(self.syntax_comment)
    }
}

/// Helper functions for common style patterns.
pub mod helpers {
    use super::*;

    /// Create a style for selected items in a list.
    pub fn selected_style(theme: &Theme) -> Style {
        Style::default()
            .fg(theme.highlight_fg)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Create a style for focused widgets.
    pub fn focused_style(theme: &Theme) -> Style {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Create a muted/secondary text style.
    pub fn muted_style(theme: &Theme) -> Style {
        Style::default().fg(theme.text_dim)
    }

    /// Create a primary action button style.
    pub fn primary_button_style(theme: &Theme) -> Style {
        Style::default()
            .fg(theme.background)
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Create a secondary button style.
    pub fn secondary_button_style(theme: &Theme) -> Style {
        Style::default().fg(theme.text).bg(theme.highlight_bg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_config::{ColorTheme, Theme};

    #[test]
    fn test_theme_ext_text() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.text();
        assert_eq!(style.fg, Some(theme.text));
    }

    #[test]
    fn test_theme_ext_title() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.title();
        assert_eq!(style.fg, Some(theme.accent));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_theme_ext_border() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.border();
        assert_eq!(style.fg, Some(theme.border));
    }

    #[test]
    fn test_theme_ext_border_focused() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.border_focused();
        assert_eq!(style.fg, Some(theme.accent));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_theme_ext_highlight() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.highlight();
        assert_eq!(style.fg, Some(theme.highlight_fg));
        assert_eq!(style.bg, Some(theme.highlight_bg));
    }

    #[test]
    fn test_theme_ext_semantic_colors() {
        let theme = Theme::from_color_theme(ColorTheme::Default);

        assert_eq!(theme.success().fg, Some(theme.success));
        assert_eq!(theme.warning().fg, Some(theme.warning));
        assert_eq!(theme.error().fg, Some(theme.error));
        assert_eq!(theme.info().fg, Some(theme.info));
        assert_eq!(theme.disabled().fg, Some(theme.disabled));
    }

    #[test]
    fn test_theme_ext_table_header() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = theme.table_header();
        assert_eq!(style.fg, Some(theme.table_header_fg));
        assert_eq!(style.bg, Some(theme.table_header_bg));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_theme_ext_syntax_styles() {
        let theme = Theme::from_color_theme(ColorTheme::Default);

        assert_eq!(theme.syntax_command().fg, Some(theme.syntax_command));
        assert_eq!(theme.syntax_string().fg, Some(theme.syntax_string));
        assert_eq!(theme.syntax_number().fg, Some(theme.syntax_number));
        assert_eq!(theme.syntax_comment().fg, Some(theme.syntax_comment));
    }

    #[test]
    fn test_selected_style() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = helpers::selected_style(&theme);
        assert_eq!(style.fg, Some(theme.highlight_fg));
        assert_eq!(style.bg, Some(theme.highlight_bg));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_focused_style() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = helpers::focused_style(&theme);
        assert_eq!(style.fg, Some(theme.accent));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_muted_style() {
        let theme = Theme::from_color_theme(ColorTheme::Default);
        let style = helpers::muted_style(&theme);
        assert_eq!(style.fg, Some(theme.text_dim));
    }

    #[test]
    fn test_button_styles() {
        let theme = Theme::from_color_theme(ColorTheme::Default);

        let primary = helpers::primary_button_style(&theme);
        assert_eq!(primary.fg, Some(theme.background));
        assert_eq!(primary.bg, Some(theme.accent));
        assert!(primary.add_modifier.contains(Modifier::BOLD));

        let secondary = helpers::secondary_button_style(&theme);
        assert_eq!(secondary.fg, Some(theme.text));
        assert_eq!(secondary.bg, Some(theme.highlight_bg));
    }

    #[test]
    fn test_theme_ext_all_themes() {
        // Test that theme helpers work across all themes
        for color_theme in [
            ColorTheme::Default,
            ColorTheme::Light,
            ColorTheme::Dark,
            ColorTheme::HighContrast,
        ] {
            let theme = Theme::from_color_theme(color_theme);

            // All themes should provide consistent style helpers
            let _ = theme.text();
            let _ = theme.text_dim();
            let _ = theme.title();
            let _ = theme.border();
            let _ = theme.border_focused();
            let _ = theme.highlight();
            let _ = theme.success();
            let _ = theme.warning();
            let _ = theme.error();
            let _ = theme.info();
            let _ = theme.disabled();
            let _ = theme.table_header();
            let _ = theme.syntax_command();
            let _ = theme.syntax_string();
            let _ = theme.syntax_number();
            let _ = theme.syntax_comment();
        }
    }
}
