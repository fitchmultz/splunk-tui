//! Toast notification widgets for transient feedback messages.
//!
//! This module provides a toast notification system that displays transient
//! messages in the bottom-right corner of the screen. Each toast has a unique
//! UUID, a severity level, and an automatic expiration time (TTL).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Severity level for toast notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Public API for future use
pub enum ToastLevel {
    /// Informational message
    Info,
    /// Success message
    Success,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl ToastLevel {
    /// Returns the display label for this level.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Success => "OK",
            Self::Warning => "WARN",
            Self::Error => "ERR",
        }
    }

    /// Returns the foreground color for this level.
    pub fn color(&self) -> Color {
        match self {
            Self::Info => Color::Cyan,
            Self::Success => Color::Green,
            Self::Warning => Color::Yellow,
            Self::Error => Color::Red,
        }
    }

    /// Returns the TTL (time-to-live) for this level.
    pub fn ttl(&self) -> Duration {
        match self {
            Self::Info | Self::Success | Self::Warning => Duration::from_secs(5),
            Self::Error => Duration::from_secs(10),
        }
    }

    /// Parses a toast level from a string (for deserialization).
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "info" => Some(Self::Info),
            "success" | "ok" => Some(Self::Success),
            "warning" | "warn" => Some(Self::Warning),
            "error" | "err" => Some(Self::Error),
            _ => None,
        }
    }
}

/// A single toast notification.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Public API for future use
pub struct Toast {
    /// Unique identifier for this toast
    pub id: Uuid,
    /// The message to display
    pub message: String,
    /// Severity level
    pub level: ToastLevel,
    /// When this toast was created
    pub created_at: Instant,
    /// Time-to-live before auto-expiry
    pub ttl: Duration,
}

impl Toast {
    /// Creates a new toast with the given message and level.
    pub fn new(message: String, level: ToastLevel) -> Self {
        let ttl = level.ttl();
        Self {
            id: Uuid::new_v4(),
            message,
            level,
            created_at: Instant::now(),
            ttl,
        }
    }

    /// Returns true if this toast has expired (TTL elapsed).
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }

    /// Returns the remaining time before expiry.
    #[allow(dead_code)]
    pub fn remaining(&self) -> Duration {
        self.ttl.saturating_sub(self.created_at.elapsed())
    }

    /// Creates an info toast.
    #[allow(dead_code)]
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastLevel::Info)
    }

    /// Creates a success toast.
    #[allow(dead_code)]
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastLevel::Success)
    }

    /// Creates a warning toast.
    #[allow(dead_code)]
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastLevel::Warning)
    }

    /// Creates an error toast.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastLevel::Error)
    }
}

/// Renders all active toasts in the bottom-right corner.
///
/// Toasts are stacked vertically with the most recent at the bottom.
/// Expired toasts are filtered out before rendering.
///
/// # Arguments
///
/// * `f` - The frame to render into
/// * `toasts` - Slice of toasts to render (will be filtered for non-expired)
pub fn render_toasts(f: &mut Frame, toasts: &[Toast]) {
    // Filter out expired toasts
    let active: Vec<_> = toasts.iter().filter(|t| !t.is_expired()).collect();

    if active.is_empty() {
        return;
    }

    // Calculate total height needed
    let toast_height = 3; // Each toast is 3 lines tall
    let total_height = active.len() as u16 * toast_height;
    let toast_width = 60;

    // Get the terminal area
    let area = f.area();

    // Ensure we have enough space
    if area.height < HEADER_HEIGHT + FOOTER_HEIGHT + total_height + 2
        || area.width < toast_width + 2
    {
        return;
    }

    // Position in bottom-right corner
    let toast_area = Rect {
        x: area.width.saturating_sub(toast_width + 2),
        y: area.height.saturating_sub(FOOTER_HEIGHT + total_height + 1),
        width: toast_width,
        height: total_height,
    };

    // Create vertical layout for stacking toasts
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat_n(Constraint::Length(toast_height), active.len()).collect::<Vec<_>>(),
        )
        .split(toast_area);

    // Render each toast
    for (toast, chunk) in active.iter().zip(chunks.iter()) {
        render_single_toast(f, toast, *chunk);
    }
}

/// Renders a single toast notification.
fn render_single_toast(f: &mut Frame, toast: &Toast, area: Rect) {
    let level = toast.level;
    let color = level.color();
    let label = level.label();

    // Truncate message if too long
    let max_width = area.width.saturating_sub(4) as usize;
    let message = if toast.message.len() > max_width {
        format!("{}...", &toast.message[..max_width.saturating_sub(3)])
    } else {
        toast.message.clone()
    };

    // Create the toast content
    let content = vec![Line::from(vec![
        Span::styled(
            format!(" {} ", label),
            Style::default()
                .fg(color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw(&message),
    ])];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Layout constants - must match app.rs
const HEADER_HEIGHT: u16 = 3;
const FOOTER_HEIGHT: u16 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_expiration() {
        let toast = Toast::info("Test message".to_string());
        assert!(!toast.is_expired(), "Fresh toast should not be expired");

        // Create an already-expired toast
        let mut expired_toast = Toast::info("Test".to_string());
        expired_toast.ttl = Duration::from_millis(1);
        std::thread::sleep(Duration::from_millis(10));
        assert!(expired_toast.is_expired(), "Old toast should be expired");
    }

    #[test]
    fn test_toast_remaining() {
        let toast = Toast::info("Test".to_string());
        let remaining = toast.remaining();
        assert!(
            remaining.as_secs() <= 5,
            "Remaining should be at most 5 seconds"
        );
        assert!(
            remaining.as_secs() >= 4,
            "Remaining should be at least 4 seconds"
        );
    }

    #[test]
    fn test_toast_level_colors() {
        assert_eq!(ToastLevel::Info.color(), Color::Cyan);
        assert_eq!(ToastLevel::Success.color(), Color::Green);
        assert_eq!(ToastLevel::Warning.color(), Color::Yellow);
        assert_eq!(ToastLevel::Error.color(), Color::Red);
    }

    #[test]
    fn test_toast_level_ttl() {
        assert_eq!(ToastLevel::Info.ttl(), Duration::from_secs(5));
        assert_eq!(ToastLevel::Success.ttl(), Duration::from_secs(5));
        assert_eq!(ToastLevel::Warning.ttl(), Duration::from_secs(5));
        assert_eq!(ToastLevel::Error.ttl(), Duration::from_secs(10));
    }

    #[test]
    fn test_toast_constructors() {
        let info = Toast::info("info");
        assert_eq!(info.level, ToastLevel::Info);

        let success = Toast::success("success");
        assert_eq!(success.level, ToastLevel::Success);

        let warning = Toast::warning("warning");
        assert_eq!(warning.level, ToastLevel::Warning);

        let error = Toast::error("error");
        assert_eq!(error.level, ToastLevel::Error);
    }

    #[test]
    fn test_toast_unique_ids() {
        let toast1 = Toast::info("test1");
        let toast2 = Toast::info("test2");
        assert_ne!(toast1.id, toast2.id, "Each toast should have a unique UUID");
    }

    #[test]
    fn test_toast_level_from_str() {
        assert_eq!(ToastLevel::from_str("info"), Some(ToastLevel::Info));
        assert_eq!(ToastLevel::from_str("INFO"), Some(ToastLevel::Info));
        assert_eq!(ToastLevel::from_str("success"), Some(ToastLevel::Success));
        assert_eq!(ToastLevel::from_str("ok"), Some(ToastLevel::Success));
        assert_eq!(ToastLevel::from_str("warning"), Some(ToastLevel::Warning));
        assert_eq!(ToastLevel::from_str("error"), Some(ToastLevel::Error));
        assert_eq!(ToastLevel::from_str("err"), Some(ToastLevel::Error));
        assert_eq!(ToastLevel::from_str("invalid"), None);
    }
}
