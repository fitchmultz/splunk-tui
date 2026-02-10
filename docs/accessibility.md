# Accessibility Guide

Splunk TUI provides accessibility-focused themes for users with color vision deficiencies and other visual impairments.

## Colorblind-Friendly Themes

### Available Themes

| Theme | Best For | Palette |
|-------|----------|---------|
| Default | General use | Cyan/yellow/green/red on black |
| Light | Bright environments | Blue/magenta/green/red on white |
| Dark | Low-light environments | Soft blue/orange/green/red on black |
| High Contrast | Low vision | Pure black/white, WCAG AAA compliant (7:1 ratio) |
| Deuteranopia | Red-green colorblindness | Blue/yellow distinction |
| Protanopia | Red-green colorblindness | Blue/orange distinction |
| Tritanopia | Blue-yellow colorblindness | Red/teal distinction |
| Monochrome | Complete colorblindness | Grayscale with pattern indicators |

### About Color Vision Deficiencies

- **Deuteranopia** (~1% males): Insensitivity to green light - red/green appear similar
- **Protanopia** (~1% males): Insensitivity to red light - red/green appear similar  
- **Tritanopia** (~0.01%): Insensitivity to blue light - blue/yellow appear similar
- **Monochrome** (rare): No color perception at all

The colorblind-friendly themes use distinct color palettes that remain distinguishable for each type of deficiency.

## Switching Themes

### In the TUI

1. Navigate to **Settings** (press `s` from any screen)
2. Press `t` to cycle through themes
3. A preview bar shows the success/warning/error/info colors for the current theme

### Via Configuration

Edit your settings file (typically `~/.config/splunk-tui/settings.json`):

```json
{
  "selected_theme": "deuteranopia"
}
```

Valid theme values: `default`, `light`, `dark`, `high_contrast`, `deuteranopia`, `protanopia`, `tritanopia`, `monochrome`

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `s` | Go to Settings screen |
| `t` | Cycle to next theme |

## Pattern Indicators

In addition to colors, the Monochrome theme and pattern indicators provide non-color cues:

| Status | Pattern | Description |
|--------|---------|-------------|
| Success | ● | Filled circle |
| Warning | ◐ | Half-filled circle |
| Error | ✗ | X mark |
| Info | ℹ | Info symbol |
| Unknown | ? | Question mark |

These patterns can be used alongside colors to provide redundant encoding for important status information.

## Contrast Ratios

All themes maintain minimum WCAG AA compliance (4.5:1 contrast ratio for normal text):

- **High Contrast**: 7:1 (WCAG AAA)
- **All other themes**: 4.5:1+ (WCAG AA)

The High Contrast theme uses pure black (#000000) background with pure white (#FFFFFF) text for maximum legibility.

## Terminal Compatibility

The themes use ANSI 256-color codes for broader terminal compatibility. For best results:

- Use a terminal with 256-color support
- Ensure your terminal font has good distinguishability for similar colors
- Consider increasing font size for the High Contrast theme

## Feedback

If you encounter accessibility issues or have suggestions for improvement, please open an issue on the project repository.
