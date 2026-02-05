//! Slider component for numeric input.
//!
//! An interactive slider for selecting numeric values or time ranges.
//! Supports keyboard navigation and visual feedback.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::Slider;
//!
//! let mut slider = Slider::new(0.0, 100.0)
//!     .label("Time Range")
//!     .value(50.0)
//!     .step(5.0);
//!
//! // Adjust value
//! slider.increase();
//! slider.decrease();
//!
//! // Get current value
//! let current_value = slider.actual_value();
//!
//! frame.render_widget(slider, area);
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use splunk_config::Theme;

/// A slider component for selecting numeric values.
#[derive(Debug, Clone)]
pub struct Slider {
    value: f64, // 0.0 to 1.0 normalized
    min: f64,
    max: f64,
    label: String,
    step: f64,
    show_percentage: bool,
    show_value: bool,
    thumb_char: char,
    track_char: char,
    filled_char: char,
    focused: bool,
}

impl Slider {
    /// Create a new slider with the given min and max values.
    pub fn new(min: f64, max: f64) -> Self {
        let step = (max - min) / 100.0;
        Self {
            value: 0.0,
            min,
            max,
            label: String::new(),
            step,
            show_percentage: true,
            show_value: true,
            thumb_char: '●',
            track_char: '─',
            filled_char: '━',
            focused: false,
        }
    }

    /// Set the label for the slider.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set the current value (clamped to min/max).
    pub fn value(mut self, value: f64) -> Self {
        self.set_actual_value(value);
        self
    }

    /// Set the step size for increment/decrement operations.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(0.0);
        self
    }

    /// Set whether to show the percentage.
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Set whether to show the actual value.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Set the character used for the thumb.
    pub fn thumb_char(mut self, ch: char) -> Self {
        self.thumb_char = ch;
        self
    }

    /// Set the character used for the unfilled track.
    pub fn track_char(mut self, ch: char) -> Self {
        self.track_char = ch;
        self
    }

    /// Set the character used for the filled portion.
    pub fn filled_char(mut self, ch: char) -> Self {
        self.filled_char = ch;
        self
    }

    /// Set whether the slider is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if the slider is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Get the actual value (not normalized).
    pub fn actual_value(&self) -> f64 {
        self.min + self.value * (self.max - self.min)
    }

    /// Get the normalized value (0.0 to 1.0).
    pub fn normalized_value(&self) -> f64 {
        self.value
    }

    /// Set the actual value (clamped to min/max).
    pub fn set_actual_value(&mut self, value: f64) {
        let clamped = value.clamp(self.min, self.max);
        self.value = (clamped - self.min) / (self.max - self.min);
    }

    /// Set the normalized value (0.0 to 1.0).
    pub fn set_normalized_value(&mut self, value: f64) {
        self.value = value.clamp(0.0, 1.0);
    }

    /// Increase value by one step.
    pub fn increase(&mut self) {
        let step_normalized = self.step / (self.max - self.min);
        self.value = (self.value + step_normalized).clamp(0.0, 1.0);
    }

    /// Decrease value by one step.
    pub fn decrease(&mut self) {
        let step_normalized = self.step / (self.max - self.min);
        self.value = (self.value - step_normalized).clamp(0.0, 1.0);
    }

    /// Set value to minimum.
    pub fn set_to_min(&mut self) {
        self.value = 0.0;
    }

    /// Set value to maximum.
    pub fn set_to_max(&mut self) {
        self.value = 1.0;
    }

    /// Get the minimum value.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Get the maximum value.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Get the label.
    pub fn label_text(&self) -> &str {
        &self.label
    }

    /// Render the track portion of the slider.
    fn render_track(&self, width: u16, theme: &Theme) -> Line<'_> {
        if width == 0 {
            return Line::from("");
        }

        let filled = (self.value * width as f64) as u16;
        let filled = filled.min(width);

        let track_fg = if self.focused {
            theme.accent
        } else {
            theme.text_dim
        };

        let mut spans = vec![];

        // Filled portion
        if filled > 0 {
            spans.push(Span::styled(
                self.filled_char.to_string().repeat(filled as usize),
                Style::default().fg(track_fg),
            ));
        }

        // Thumb - only if there's space
        if filled < width {
            spans.push(Span::styled(
                self.thumb_char.to_string(),
                Style::default().fg(theme.accent),
            ));

            // Unfilled portion
            let unfilled = width.saturating_sub(filled).saturating_sub(1);
            if unfilled > 0 {
                spans.push(Span::styled(
                    self.track_char.to_string().repeat(unfilled as usize),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        Line::from(spans)
    }

    /// Render the label portion of the slider.
    fn render_label(&self, theme: &Theme) -> Line<'_> {
        let mut parts = vec![];

        if !self.label.is_empty() {
            parts.push(self.label.clone());
        }

        if self.show_value {
            let value_str = format!("{:.1}", self.actual_value());
            parts.push(value_str);
        }

        if self.show_percentage {
            let pct = (self.value * 100.0) as u16;
            parts.push(format!("{}%", pct));
        }

        let label_text = if parts.len() > 1 {
            // Join with ": " except between label and value
            if !self.label.is_empty() && (self.show_value || self.show_percentage) {
                let label = parts.remove(0);
                format!("{}: {}", label, parts.join(" "))
            } else {
                parts.join(": ")
            }
        } else if parts.len() == 1 {
            parts.remove(0)
        } else {
            String::new()
        };

        Line::from(label_text).style(Style::default().fg(theme.text))
    }
}

impl Widget for Slider {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = Theme::default();

        if area.height >= 2 {
            // Render track on first line
            let track_line = self.render_track(area.width, &theme);
            buf.set_line(area.x, area.y, &track_line, area.width);

            // Render label on second line
            let label_line = self.render_label(&theme);
            buf.set_line(area.x, area.y + 1, &label_line, area.width);
        } else if area.height == 1 {
            // Single line: just track
            let track_line = self.render_track(area.width, &theme);
            buf.set_line(area.x, area.y, &track_line, area.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_creation() {
        let slider = Slider::new(0.0, 100.0);

        assert_eq!(slider.min(), 0.0);
        assert_eq!(slider.max(), 100.0);
        assert_eq!(slider.actual_value(), 0.0);
        assert!(!slider.is_focused());
    }

    #[test]
    fn test_slider_builder() {
        let slider = Slider::new(0.0, 100.0)
            .label("Test")
            .value(50.0)
            .step(10.0)
            .show_percentage(false)
            .show_value(false)
            .thumb_char('■')
            .track_char('░');

        assert_eq!(slider.label_text(), "Test");
        assert_eq!(slider.actual_value(), 50.0);
        assert!(!slider.show_percentage);
        assert!(!slider.show_value);
        assert_eq!(slider.thumb_char, '■');
        assert_eq!(slider.track_char, '░');
    }

    #[test]
    fn test_slider_value_clamping() {
        let mut slider = Slider::new(0.0, 100.0).value(150.0);
        assert_eq!(slider.actual_value(), 100.0);

        slider.set_actual_value(-50.0);
        assert_eq!(slider.actual_value(), 0.0);

        slider.set_actual_value(75.0);
        assert_eq!(slider.actual_value(), 75.0);
    }

    #[test]
    fn test_slider_normalized_value() {
        let mut slider = Slider::new(0.0, 100.0);

        slider.set_normalized_value(0.5);
        assert_eq!(slider.normalized_value(), 0.5);
        assert_eq!(slider.actual_value(), 50.0);

        slider.set_normalized_value(1.5);
        assert_eq!(slider.normalized_value(), 1.0);

        slider.set_normalized_value(-0.5);
        assert_eq!(slider.normalized_value(), 0.0);
    }

    #[test]
    fn test_slider_step() {
        let mut slider = Slider::new(0.0, 100.0).step(10.0);
        slider.set_actual_value(0.0);

        slider.increase();
        assert_eq!(slider.actual_value(), 10.0);

        slider.increase();
        assert_eq!(slider.actual_value(), 20.0);

        slider.decrease();
        assert_eq!(slider.actual_value(), 10.0);
    }

    #[test]
    fn test_slider_step_does_not_exceed_bounds() {
        let mut slider = Slider::new(0.0, 100.0).step(50.0);
        slider.set_actual_value(80.0);

        slider.increase();
        assert_eq!(slider.actual_value(), 100.0);

        slider.increase();
        assert_eq!(slider.actual_value(), 100.0); // Stays at max

        slider.set_actual_value(20.0);
        slider.decrease();
        assert_eq!(slider.actual_value(), 0.0);

        slider.decrease();
        assert_eq!(slider.actual_value(), 0.0); // Stays at min
    }

    #[test]
    fn test_slider_focus() {
        let mut slider = Slider::new(0.0, 100.0);

        assert!(!slider.is_focused());
        slider.set_focused(true);
        assert!(slider.is_focused());
        slider.set_focused(false);
        assert!(!slider.is_focused());
    }

    #[test]
    fn test_slider_set_to_min_max() {
        let mut slider = Slider::new(0.0, 100.0);
        slider.set_actual_value(50.0);

        slider.set_to_min();
        assert_eq!(slider.actual_value(), 0.0);

        slider.set_to_max();
        assert_eq!(slider.actual_value(), 100.0);
    }

    #[test]
    fn test_slider_percentage_calculation() {
        let slider = Slider::new(0.0, 100.0).value(75.0);
        assert_eq!((slider.normalized_value() * 100.0) as u16, 75);

        let slider = Slider::new(0.0, 200.0).value(100.0);
        assert_eq!((slider.normalized_value() * 100.0) as u16, 50);
    }

    #[test]
    fn test_slider_widget_render() {
        let slider = Slider::new(0.0, 100.0).value(50.0);

        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 2));
        slider.render(Rect::new(0, 0, 40, 2), &mut buf);

        // Buffer should have content
        let content: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_slider_widget_render_single_line() {
        let slider = Slider::new(0.0, 100.0).value(50.0);

        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        slider.render(Rect::new(0, 0, 40, 1), &mut buf);

        // Should render without panic on single line
        let content: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_slider_zero_width() {
        let slider = Slider::new(0.0, 100.0).value(50.0);

        let mut buf = Buffer::empty(Rect::new(0, 0, 0, 2));
        slider.render(Rect::new(0, 0, 0, 2), &mut buf);

        // Should handle zero width gracefully
    }

    #[test]
    fn test_slider_custom_chars() {
        let slider = Slider::new(0.0, 100.0)
            .thumb_char('█')
            .filled_char('█')
            .track_char('░');

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        slider.render(Rect::new(0, 0, 20, 1), &mut buf);

        // Should render with custom characters
    }

    #[test]
    fn test_slider_non_zero_min() {
        let mut slider = Slider::new(10.0, 20.0);

        slider.set_actual_value(15.0);
        assert_eq!(slider.actual_value(), 15.0);
        assert_eq!(slider.normalized_value(), 0.5);

        slider.set_to_max();
        assert_eq!(slider.actual_value(), 20.0);
    }
}
