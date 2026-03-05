//! Test helpers for TUI testing.
//!
//! Provides utility functions for simulating keyboard input and creating
//! test fixtures for the TUI application.

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use splunk_client::models::{Index, LogEntry, LogLevel, SearchJobStatus, User, UserType};
use splunk_tui::{App, ConnectionContext};

/// Test harness for TUI rendering with a mock terminal.
pub struct TuiHarness {
    pub app: App,
    pub terminal: Terminal<TestBackend>,
}

impl TuiHarness {
    /// Create a new test harness with the given terminal dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).expect("Failed to create terminal");
        let app = App::new(None, ConnectionContext::default());
        Self { app, terminal }
    }

    /// Render the current app state and return the buffer contents.
    pub fn render(&mut self) -> String {
        self.terminal
            .draw(|f| self.app.render(f))
            .expect("Failed to render");
        buffer_to_string(self.terminal.backend().buffer())
    }
}

/// Convert a ratatui Buffer to a string for snapshot testing.
pub fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area();
    let mut output = String::new();

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &buffer[(x, y)];
            output.push(cell.symbol().chars().next().unwrap_or(' '));
        }
        if y < area.bottom() - 1 {
            output.push('\n');
        }
    }

    output
}

/// Create mock user data for testing.
pub fn create_mock_users() -> Vec<User> {
    vec![
        User {
            name: "admin".to_string(),
            realname: Some("System Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some(UserType::Splunk),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string(), "can_delete".to_string()],
            last_successful_login: Some(1736956200), // 2024-01-15 10:30:00 UTC
        },
        User {
            name: "power_user".to_string(),
            realname: Some("Power User".to_string()),
            email: Some("power@example.com".to_string()),
            user_type: Some(UserType::Splunk),
            default_app: Some("search".to_string()),
            roles: vec!["power".to_string()],
            last_successful_login: Some(1736870400), // 2024-01-14 10:00:00 UTC
        },
        User {
            name: "user_no_roles".to_string(),
            realname: Some("Limited User".to_string()),
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
    ]
}

/// Create mock job data for testing.
pub fn create_mock_jobs() -> Vec<SearchJobStatus> {
    vec![
        SearchJobStatus {
            sid: "scheduler_admin_search_1234567890".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 5.23,
            disk_usage: 2048,
            scan_count: 1500,
            event_count: 500,
            result_count: 100,
            cursor_time: Some("2024-01-15T10:30:00.000Z".to_string()),
            priority: Some(5),
            label: Some("Scheduled search".to_string()),
        },
        SearchJobStatus {
            sid: "admin_search_9876543210".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.65,
            run_duration: 12.45,
            disk_usage: 5120,
            scan_count: 5000,
            event_count: 2000,
            result_count: 450,
            cursor_time: Some("2024-01-15T10:29:00.000Z".to_string()),
            priority: Some(3),
            label: Some("Ad-hoc search".to_string()),
        },
    ]
}

/// Create mock index data for testing.
pub fn create_mock_index() -> Index {
    Index {
        name: "test_index".to_string(),
        max_total_data_size_mb: Some(100000),
        current_db_size_mb: 50000,
        total_event_count: 1000000,
        max_warm_db_count: Some(300),
        max_hot_buckets: Some("10".to_string()),
        frozen_time_period_in_secs: Some(2592000),
        cold_db_path: Some("/opt/splunk/cold".to_string()),
        home_path: Some("/opt/splunk/var/lib/splunk/test_index/db".to_string()),
        thawed_path: Some("/opt/splunk/thawed".to_string()),
        cold_to_frozen_dir: Some("/opt/splunk/frozen".to_string()),
        primary_index: Some(false),
    }
}

/// Create mock log entries for testing.
pub fn create_mock_logs() -> Vec<LogEntry> {
    vec![
        LogEntry {
            time: "2024-01-15T10:30:00.000Z".to_string(),
            index_time: "2024-01-15T10:30:01.000Z".to_string(),
            serial: Some(1),
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "some metrics log message".to_string(),
        },
        LogEntry {
            time: "2024-01-15T10:29:00.000Z".to_string(),
            index_time: "2024-01-15T10:29:01.000Z".to_string(),
            serial: Some(2),
            level: LogLevel::Error,
            component: "DateParser".to_string(),
            message: "failed to parse date".to_string(),
        },
    ]
}

/// Create a character key event.
pub fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// Create an Enter key event.
pub fn enter_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}

/// Create an Escape key event.
pub fn esc_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
}

/// Create a Down arrow key event.
pub fn down_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
}

/// Create an Up arrow key event.
pub fn up_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
}

/// Create a Page Down key event.
pub fn page_down_key() -> KeyEvent {
    KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)
}

/// Create a Page Up key event.
pub fn page_up_key() -> KeyEvent {
    KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)
}

/// Create a Home key event.
pub fn home_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
}

/// Create an End key event.
pub fn end_key() -> KeyEvent {
    KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
}

/// Create a Backspace key event.
pub fn backspace_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
}

/// Create a Delete key event.
pub fn delete_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)
}

/// Create a Left arrow key event.
pub fn left_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
}

/// Create a Right arrow key event.
pub fn right_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
}

/// Create a Tab key event.
pub fn tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
}

/// Create a Shift+Tab (BackTab) key event.
pub fn shift_tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)
}

/// Create a Ctrl+Tab key event.
pub fn ctrl_tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL)
}

/// Create a Ctrl+Shift+Tab key event.
pub fn ctrl_shift_tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::BackTab, KeyModifiers::CONTROL)
}

/// Create a Ctrl+char key event.
pub fn ctrl_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// Create a Shift+char key event.
pub fn shift_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
}

/// Create a key event with explicit KeyEventKind.
pub fn key_with_kind(code: KeyCode, kind: crossterm::event::KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: crossterm::event::KeyEventState::NONE,
    }
}

/// Create a Release key event (used to test filtering).
pub fn release_key(c: char) -> KeyEvent {
    key_with_kind(KeyCode::Char(c), crossterm::event::KeyEventKind::Release)
}

/// Create a Repeat key event (used to test filtering).
pub fn repeat_key(c: char) -> KeyEvent {
    key_with_kind(KeyCode::Char(c), crossterm::event::KeyEventKind::Repeat)
}

/// Create a mouse click event.
pub fn mouse_click(col: u16, row: u16) -> crossterm::event::MouseEvent {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

/// Create error details from a string for testing.
pub fn error_details_from_string(error: &str) -> splunk_tui::error_details::ErrorDetails {
    splunk_tui::error_details::ErrorDetails::from_error_string(error)
}
