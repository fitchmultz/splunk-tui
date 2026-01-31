//! Rendering logic for the TUI app.
//!
//! Responsibilities:
//! - Render the main app layout (header, content, footer)
//! - Dispatch to screen-specific renderers
//! - Render jobs and job details screens
//!
//! Non-responsibilities:
//! - Does NOT handle input
//! - Does NOT mutate app state (except for ListState/TableState selection)

use crate::app::App;
use crate::app::state::{CurrentScreen, FOOTER_HEIGHT, HEADER_HEIGHT, HealthState};
use crate::input::keymap::footer_hints;
use crate::ui::popup::PopupType;
use crate::ui::screens::{
    apps, cluster, configs, forwarders, health, indexes, inputs, kvstore, license, multi_instance,
    overview, saved_searches, search, search_peers, settings, users,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

impl App {
    /// Render the application UI.
    pub fn render(&mut self, f: &mut Frame) {
        self.last_area = f.area();

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(HEADER_HEIGHT),
                    Constraint::Min(0),
                    Constraint::Length(FOOTER_HEIGHT),
                ]
                .as_ref(),
            )
            .split(f.area());

        // Header
        let theme = self.theme;

        // Build health indicator span
        let health_indicator = match self.health_state {
            HealthState::Healthy => Span::styled("[+]", Style::default().fg(theme.health_healthy)),
            HealthState::Unhealthy => {
                Span::styled("[!]", Style::default().fg(theme.health_unhealthy))
            }
            HealthState::Unknown => Span::styled("[?]", Style::default().fg(theme.health_unknown)),
        };

        let health_label = match self.health_state {
            HealthState::Healthy => "Healthy",
            HealthState::Unhealthy => "Unhealthy",
            HealthState::Unknown => "Unknown",
        };

        let health_label_style = match self.health_state {
            HealthState::Healthy => Style::default().fg(theme.health_healthy),
            HealthState::Unhealthy => Style::default().fg(theme.health_unhealthy),
            HealthState::Unknown => Style::default().fg(theme.health_unknown),
        };

        // Build connection context line for header (RQ-0134)
        let connection_line = self.format_connection_context();

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "Splunk TUI",
                    Style::default()
                        .fg(theme.title)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - "),
                Span::styled(
                    match self.current_screen {
                        CurrentScreen::Search => "Search",
                        CurrentScreen::Indexes => "Indexes",
                        CurrentScreen::Cluster => "Cluster",
                        CurrentScreen::Jobs => "Jobs",
                        CurrentScreen::JobInspect => "Job Details",
                        CurrentScreen::Health => "Health",
                        CurrentScreen::License => "License",
                        CurrentScreen::Kvstore => "KVStore",
                        CurrentScreen::SavedSearches => "Saved Searches",
                        CurrentScreen::InternalLogs => "Internal Logs",
                        CurrentScreen::Apps => "Apps",
                        CurrentScreen::Users => "Users",
                        CurrentScreen::SearchPeers => "Search Peers",
                        CurrentScreen::Inputs => "Data Inputs",
                        CurrentScreen::Configs => "Config Files",
                        CurrentScreen::FiredAlerts => "Fired Alerts",
                        CurrentScreen::Forwarders => "Forwarders",
                        CurrentScreen::Settings => "Settings",
                        CurrentScreen::Overview => "Overview",
                        CurrentScreen::MultiInstance => "Multi-Instance",
                    },
                    Style::default().fg(theme.accent),
                ),
                Span::raw(" | "),
                health_indicator,
                Span::raw(" "),
                Span::styled(health_label, health_label_style),
            ]),
            Line::from(connection_line),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
        f.render_widget(header, chunks[0]);

        // Main content
        self.render_content(f, chunks[1]);

        // Footer with status and per-screen hints
        let footer_text = self.build_footer_text(theme);
        let footer = Paragraph::new(footer_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
        f.render_widget(footer, chunks[2]);

        // Render toasts
        crate::ui::toast::render_toasts(f, &self.toasts, self.current_error.is_some(), &self.theme);

        // Render popup if active (on top of toasts)
        if let Some(ref popup) = self.popup {
            crate::ui::popup::render_popup(f, popup, &self.theme, self);
        }

        // Render error details popup if active
        if let Some(crate::ui::popup::Popup {
            kind: PopupType::ErrorDetails,
            ..
        }) = &self.popup
            && let Some(error) = &self.current_error
        {
            crate::ui::error_details::render_error_details(f, error, self, &self.theme);
        }

        // Render index details popup if active
        if let Some(crate::ui::popup::Popup {
            kind: PopupType::IndexDetails,
            ..
        }) = &self.popup
        {
            crate::ui::index_details::render_index_details(f, self, &self.theme);
        }
    }

    fn render_content(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        match self.current_screen {
            CurrentScreen::Search => {
                search::render_search(
                    f,
                    area,
                    search::SearchRenderConfig {
                        search_input: &self.search_input,
                        search_cursor_position: self.search_cursor_position,
                        is_query_focused: matches!(
                            self.search_input_mode,
                            crate::app::state::SearchInputMode::QueryFocused
                        ),
                        search_status: &self.search_status,
                        loading: self.loading,
                        progress: self.progress,
                        search_results: &self.search_results,
                        search_scroll_offset: self.search_scroll_offset,
                        search_results_total_count: self.search_results_total_count,
                        search_has_more_results: self.search_has_more_results,
                        theme: &self.theme,
                        spl_validation_state: &self.spl_validation_state,
                        spl_validation_pending: self.spl_validation_pending,
                    },
                );
            }
            CurrentScreen::Indexes => {
                indexes::render_indexes(
                    f,
                    area,
                    indexes::IndexesRenderConfig {
                        loading: self.loading,
                        indexes: self.indexes.as_deref(),
                        state: &mut self.indexes_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Cluster => {
                cluster::render_cluster(
                    f,
                    area,
                    cluster::ClusterRenderConfig {
                        loading: self.loading,
                        cluster_info: self.cluster_info.as_ref(),
                        cluster_peers: self.cluster_peers.as_deref(),
                        view_mode: self.cluster_view_mode,
                        peers_state: &mut self.cluster_peers_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Jobs => self.render_jobs(f, area),
            CurrentScreen::JobInspect => self.render_job_details(f, area),
            CurrentScreen::Health => {
                health::render_health(
                    f,
                    area,
                    health::HealthRenderConfig {
                        loading: self.loading,
                        health_info: self.health_info.as_ref(),
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::License => {
                license::render_license(
                    f,
                    area,
                    license::LicenseRenderConfig {
                        loading: self.loading,
                        license_info: self.license_info.as_ref(),
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Kvstore => {
                kvstore::render_kvstore(
                    f,
                    area,
                    kvstore::KvstoreRenderConfig {
                        loading: self.loading,
                        kvstore_status: self.kvstore_status.as_ref(),
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::SavedSearches => {
                saved_searches::render_saved_searches(
                    f,
                    area,
                    saved_searches::SavedSearchesRenderConfig {
                        loading: self.loading,
                        saved_searches: self.saved_searches.as_deref(),
                        state: &mut self.saved_searches_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::InternalLogs => self.render_internal_logs(f, area),
            CurrentScreen::Apps => {
                apps::render_apps(
                    f,
                    area,
                    apps::AppsRenderConfig {
                        loading: self.loading,
                        apps: self.apps.as_deref(),
                        state: &mut self.apps_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Users => {
                users::render_users(
                    f,
                    area,
                    users::UsersRenderConfig {
                        loading: self.loading,
                        users: self.users.as_deref(),
                        state: &mut self.users_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::SearchPeers => {
                search_peers::render_search_peers(
                    f,
                    area,
                    search_peers::SearchPeersRenderConfig {
                        loading: self.loading,
                        search_peers: self.search_peers.as_deref(),
                        peers_state: &mut self.search_peers_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Settings => {
                settings::render_settings(
                    f,
                    area,
                    settings::SettingsRenderConfig {
                        auto_refresh: self.auto_refresh,
                        sort_column: self.sort_state.column.as_str(),
                        sort_direction: self.sort_state.direction.as_str(),
                        search_history_count: self.search_history.len(),
                        profile_info: self.profile_name.as_deref(),
                        selected_theme: self.color_theme,
                        theme: &self.theme,
                        earliest_time: &self.search_defaults.earliest_time,
                        latest_time: &self.search_defaults.latest_time,
                        max_results: self.search_defaults.max_results,
                        internal_logs_count: self.internal_logs_defaults.count,
                        internal_logs_earliest: &self.internal_logs_defaults.earliest_time,
                    },
                );
            }
            CurrentScreen::Overview => {
                overview::render_overview(
                    f,
                    area,
                    overview::OverviewRenderConfig {
                        loading: self.loading,
                        overview_data: self.overview_data.as_ref(),
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::MultiInstance => {
                multi_instance::render_multi_instance(
                    f,
                    area,
                    multi_instance::MultiInstanceRenderConfig {
                        loading: self.loading,
                        data: self.multi_instance_data.as_ref(),
                        selected_index: self.multi_instance_selected_index,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Inputs => {
                inputs::render_inputs(
                    f,
                    area,
                    inputs::InputsRenderConfig {
                        loading: self.loading,
                        inputs: self.inputs.as_deref(),
                        state: &mut self.inputs_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Configs => {
                configs::render_configs(
                    f,
                    area,
                    configs::ConfigsRenderConfig {
                        loading: self.loading,
                        config_files: self.config_files.as_deref(),
                        selected_file: self.selected_config_file.as_deref(),
                        stanzas: self.config_stanzas.as_deref(),
                        selected_stanza: self.selected_stanza.as_ref(),
                        view_mode: self.config_view_mode,
                        files_state: &mut self.config_files_state,
                        stanzas_state: &mut self.config_stanzas_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::FiredAlerts => {
                crate::ui::screens::alerts::render_fired_alerts(
                    f,
                    area,
                    crate::ui::screens::alerts::FiredAlertsRenderConfig {
                        loading: self.loading,
                        fired_alerts: self.fired_alerts.as_deref(),
                        state: &mut self.fired_alerts_state,
                        theme: &self.theme,
                    },
                );
            }
            CurrentScreen::Forwarders => {
                forwarders::render_forwarders(
                    f,
                    area,
                    forwarders::ForwardersRenderConfig {
                        loading: self.loading,
                        forwarders: self.forwarders.as_deref(),
                        forwarders_state: &mut self.forwarders_state,
                        theme: &self.theme,
                    },
                );
            }
        }
    }

    fn render_jobs(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        use crate::ui::screens::jobs;

        if self.loading && self.jobs.is_none() {
            let loading = Paragraph::new("Loading jobs...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.auto_refresh {
                            "Search Jobs [AUTO]"
                        } else {
                            "Search Jobs"
                        }),
                )
                .alignment(Alignment::Center);
            f.render_widget(loading, area);
            return;
        }

        let jobs = match &self.jobs {
            Some(j) => j,
            None => {
                let placeholder = Paragraph::new(if self.auto_refresh {
                    "No jobs loaded. Press 'r' to refresh, 'a' to toggle auto-refresh."
                } else {
                    "No jobs loaded. Press 'r' to refresh."
                })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.auto_refresh {
                            "Search Jobs [AUTO]"
                        } else {
                            "Search Jobs"
                        }),
                )
                .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
                return;
            }
        };

        // Get the filtered and sorted jobs (computed by App for selection consistency)
        let filtered_jobs: Vec<&splunk_client::models::SearchJobStatus> = self
            .filtered_job_indices
            .iter()
            .filter_map(|&i| jobs.get(i))
            .collect();

        jobs::render_jobs(
            f,
            area,
            jobs::JobsRenderConfig {
                jobs: &filtered_jobs,
                state: &mut self.jobs_state,
                auto_refresh: self.auto_refresh,
                filter: &self.search_filter,
                filter_input: &self.filter_input,
                is_filtering: self.is_filtering,
                sort_column: self.sort_state.column,
                sort_direction: self.sort_state.direction,
                selected_jobs: &self.selected_jobs,
                theme: &self.theme,
            },
        );
    }

    fn render_job_details(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        use crate::ui::screens::job_details;

        // Get the selected job (accounting for filter/sort)
        let job = self.get_selected_job();

        match job {
            Some(job) => {
                job_details::render_details(f, area, job, &self.theme);
            }
            None => {
                let placeholder = Paragraph::new("No job selected or jobs not loaded.")
                    .block(Block::default().borders(Borders::ALL).title("Job Details"))
                    .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
            }
        }
    }

    fn render_internal_logs(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        use crate::ui::screens::internal_logs;

        internal_logs::render_internal_logs(
            f,
            area,
            internal_logs::InternalLogsRenderConfig {
                loading: self.loading,
                logs: self.internal_logs.as_deref(),
                state: &mut self.internal_logs_state,
                auto_refresh: self.auto_refresh,
                theme: &self.theme,
            },
        );
    }

    /// Format connection context for header display (RQ-0134).
    ///
    /// Returns a vector of spans representing:
    /// - profile@base_url (or just base_url if no profile)
    /// - auth mode (token or session)
    /// - server version (if available)
    ///
    /// Long URLs are truncated to fit the terminal width.
    fn format_connection_context(&self) -> Vec<Span<'_>> {
        let theme = self.theme;
        let mut spans = Vec::new();

        // Build connection string: profile@base_url or just base_url
        let conn_str = match (&self.profile_name, &self.base_url) {
            (Some(profile), Some(url)) => format!("{}@{}", profile, Self::truncate_url(url, 40)),
            (None, Some(url)) => Self::truncate_url(url, 40),
            _ => "Connecting...".to_string(),
        };

        spans.push(Span::styled(conn_str, Style::default().fg(theme.text)));

        // Add auth mode if available
        if let Some(ref auth) = self.auth_mode {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(
                auth.clone(),
                Style::default().fg(theme.accent),
            ));
        }

        // Add server version if available
        if let Some(ref version) = self.server_version {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(
                format!("v{}", version),
                Style::default().fg(theme.success),
            ));
        }

        spans
    }

    /// Truncate URL for display, keeping the most significant parts.
    ///
    /// For long URLs, shows the end (domain:port) with ellipsis prefix.
    fn truncate_url(url: &str, max_len: usize) -> String {
        if url.len() <= max_len {
            url.to_string()
        } else {
            // Show ellipsis + end of URL (domain is more important than protocol)
            let start = url.len().saturating_sub(max_len - 3);
            format!("...{}", &url[start..])
        }
    }

    /// Build footer text with navigation hints, screen-specific hints, and controls.
    ///
    /// Layout: [Loading] | Tab:Next | Shift+Tab:Prev | [Screen Hints] | ?:Help | q:Quit
    /// Handles narrow terminals by truncating screen hints with ellipsis.
    fn build_footer_text(&self, theme: splunk_config::Theme) -> Vec<Line<'_>> {
        let hints = footer_hints(self.current_screen);
        let available_width = self.last_area.width as usize;

        // Calculate minimum required width for fixed elements
        let loading_width = if self.loading {
            format!(" Loading... {:.0}% |", self.progress * 100.0).len()
        } else {
            0
        };
        let nav_width = " Tab:Next Screen | Shift+Tab:Previous Screen ".len();
        let help_quit_width = " ?:Help | q:Quit ".len();

        let fixed_width = loading_width + nav_width + help_quit_width;
        let hints_available_width = available_width.saturating_sub(fixed_width);

        // Build screen hints string
        let hints_text = if hints.is_empty() {
            String::new()
        } else {
            let mut text = String::new();
            for (key, desc) in hints {
                let hint = format!(" {}:{}", key, desc);
                // Check if adding this hint would exceed available width
                if text.len() + hint.len() > hints_available_width && !text.is_empty() {
                    // Truncate with ellipsis
                    text.push_str(" ...");
                    break;
                }
                text.push_str(&hint);
            }
            text
        };

        // Build the footer line
        let mut spans = Vec::new();

        // Loading indicator (if active)
        if self.loading {
            spans.push(Span::styled(
                format!(" Loading... {:.0}% ", self.progress * 100.0),
                Style::default().fg(theme.warning),
            ));
            spans.push(Span::raw("|"));
        }

        // Navigation hints
        spans.push(Span::raw(" Tab:Next Screen | Shift+Tab:Previous Screen "));

        // Screen-specific hints
        if !hints_text.is_empty() {
            spans.push(Span::raw("|"));
            spans.push(Span::styled(hints_text, Style::default().fg(theme.accent)));
        }

        // Help and Quit
        spans.push(Span::raw("|"));
        spans.push(Span::styled(" ?:Help ", Style::default().fg(theme.success)));
        spans.push(Span::raw("|"));
        spans.push(Span::styled(" q:Quit ", Style::default().fg(theme.error)));

        vec![Line::from(spans)]
    }
}
