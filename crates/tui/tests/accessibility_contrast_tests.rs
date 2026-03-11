//! Purpose: Automated accessibility contrast validation for shipped TUI color themes.
//! Responsibilities: Compute luminance/contrast ratios for critical theme color pairs.
//! Scope: Theme palette validation only; does not inspect live terminal rendering.
//! Usage: Run via `cargo test -p splunk-tui --test accessibility_contrast_tests` or `make tui-accessibility`.
//! Invariants/Assumptions: Contrast checks are deterministic and operate on normalized RGB mappings.

use ratatui::style::Color;
use splunk_config::ColorTheme;
use splunk_tui::theme::Theme;

const MIN_TEXT_CONTRAST: f64 = 4.5;
const MIN_HIGHLIGHT_CONTRAST: f64 = 3.0;
const MIN_HEADER_CONTRAST: f64 = 2.0;
const MIN_TITLE_CONTRAST: f64 = 2.5;

#[test]
fn critical_theme_pairs_meet_minimum_contrast_thresholds() {
    for color_theme in [
        ColorTheme::Default,
        ColorTheme::Light,
        ColorTheme::Dark,
        ColorTheme::HighContrast,
        ColorTheme::Deuteranopia,
        ColorTheme::Protanopia,
        ColorTheme::Tritanopia,
        ColorTheme::Monochrome,
    ] {
        let theme = Theme::from_color_theme(color_theme);

        assert_pair_contrast(
            color_theme,
            "text/background",
            theme.text,
            theme.background,
            MIN_TEXT_CONTRAST,
        );
        assert_pair_contrast(
            color_theme,
            "highlight_fg/highlight_bg",
            theme.highlight_fg,
            theme.highlight_bg,
            MIN_HIGHLIGHT_CONTRAST,
        );
        assert_pair_contrast(
            color_theme,
            "table_header_fg/table_header_bg",
            theme.table_header_fg,
            theme.table_header_bg,
            MIN_HEADER_CONTRAST,
        );
        assert_pair_contrast(
            color_theme,
            "title/background",
            theme.title,
            theme.background,
            MIN_TITLE_CONTRAST,
        );
    }
}

fn assert_pair_contrast(
    color_theme: ColorTheme,
    label: &str,
    foreground: Color,
    background: Color,
    minimum_ratio: f64,
) {
    let fg = color_to_rgb(foreground);
    let bg = color_to_rgb(background);
    let ratio = contrast_ratio(fg, bg);

    assert!(
        ratio >= minimum_ratio,
        "Theme {} pair {} below contrast threshold: ratio={:.2}, required={:.2}, fg={:?}, bg={:?}",
        color_theme,
        label,
        ratio,
        minimum_ratio,
        foreground,
        background
    );
}

fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
    let fg_l = relative_luminance(fg);
    let bg_l = relative_luminance(bg);
    let (lighter, darker) = if fg_l >= bg_l {
        (fg_l, bg_l)
    } else {
        (bg_l, fg_l)
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let r = to_linear(r);
    let g = to_linear(g);
    let b = to_linear(b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn to_linear(channel: u8) -> f64 {
    let value = channel as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Reset => (0, 0, 0),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        Color::Indexed(index) => indexed_color_to_rgb(index),
        Color::Rgb(r, g, b) => (r, g, b),
    }
}

fn indexed_color_to_rgb(index: u8) -> (u8, u8, u8) {
    const ANSI_16: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    match index {
        0..=15 => ANSI_16[index as usize],
        16..=231 => {
            let value = index - 16;
            let r = value / 36;
            let g = (value % 36) / 6;
            let b = value % 6;
            (cube_component(r), cube_component(g), cube_component(b))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

fn cube_component(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}
