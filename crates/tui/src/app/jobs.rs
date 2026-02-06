//! Jobs screen specific logic for the TUI app.
//!
//! Responsibilities:
//! - Filter and sort job lists
//! - Manage job selection with filtering
//! - Compare jobs for sorting
//!
//! Does NOT handle:
//! - Does NOT handle job API operations
//! - Does NOT render the jobs table

use crate::app::App;
use crate::app::state::{SortColumn, SortDirection};
use splunk_client::models::SearchJobStatus;

impl App {
    /// Rebuild the filtered job indices based on the current filter and jobs.
    /// The indices are sorted according to the current sort settings.
    pub(crate) fn rebuild_filtered_indices(&mut self) {
        let Some(jobs) = &self.jobs else {
            self.filtered_job_indices.clear();
            return;
        };

        // First filter the jobs
        let mut filtered_and_sorted: Vec<usize> = if let Some(filter) = &self.search_filter {
            let lower_filter = filter.to_lowercase();
            jobs.iter()
                .enumerate()
                .filter(|(_, job)| {
                    job.sid.to_lowercase().contains(&lower_filter)
                        || (job.is_done && "done".contains(&lower_filter))
                        || (!job.is_done && "running".contains(&lower_filter))
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            // No filter: all jobs are visible
            (0..jobs.len()).collect()
        };

        // Then sort the filtered indices using the same comparison logic as jobs.rs
        filtered_and_sorted.sort_by(|&a, &b| {
            let job_a = &jobs[a];
            let job_b = &jobs[b];
            self.compare_jobs_for_sort(job_a, job_b)
        });

        self.filtered_job_indices = filtered_and_sorted;

        // Clamp selection to filtered list length
        let filtered_len = self.filtered_job_indices.len();
        if let Some(selected) = self.jobs_state.selected() {
            if filtered_len == 0 {
                self.jobs_state.select(None);
            } else if selected >= filtered_len {
                self.jobs_state.select(Some(filtered_len - 1));
            }
        }
    }

    /// Compare two jobs for sorting based on current sort settings.
    /// Matches the logic in jobs.rs::compare_jobs.
    pub(crate) fn compare_jobs_for_sort(
        &self,
        a: &SearchJobStatus,
        b: &SearchJobStatus,
    ) -> std::cmp::Ordering {
        let ordering = match self.sort_state.column {
            SortColumn::Sid => a.sid.cmp(&b.sid),
            SortColumn::Status => {
                // Sort by is_done first, then by progress
                match (a.is_done, b.is_done) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a
                        .done_progress
                        .partial_cmp(&b.done_progress)
                        .unwrap_or(std::cmp::Ordering::Equal),
                }
            }
            SortColumn::Duration => a
                .run_duration
                .partial_cmp(&b.run_duration)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::Results => a.result_count.cmp(&b.result_count),
            SortColumn::Events => a.event_count.cmp(&b.event_count),
        };

        match self.sort_state.direction {
            SortDirection::Asc => ordering,
            SortDirection::Desc => ordering.reverse(),
        }
    }

    /// Get the currently selected job, accounting for any active filter.
    pub fn get_selected_job(&self) -> Option<&SearchJobStatus> {
        let selected = self.jobs_state.selected()?;
        let original_idx = self.filtered_job_indices.get(selected)?;
        self.jobs.as_ref()?.get(*original_idx)
    }

    /// Get the length of the filtered jobs list.
    pub(crate) fn filtered_jobs_len(&self) -> usize {
        self.filtered_job_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::{SortColumn, SortDirection};

    fn create_test_job(sid: &str, is_done: bool, run_duration: f64) -> SearchJobStatus {
        SearchJobStatus {
            sid: sid.to_string(),
            is_done,
            is_finalized: is_done,
            done_progress: if is_done { 1.0 } else { 0.5 },
            run_duration,
            cursor_time: None,
            scan_count: 0,
            event_count: 0,
            result_count: 0,
            disk_usage: 0,
            priority: None,
            label: None,
        }
    }

    #[test]
    fn test_compare_jobs_for_sort_by_sid_asc() {
        let app = App::new(None, ConnectionContext::default());
        let job_a = create_test_job("job1", true, 1.0);
        let job_b = create_test_job("job2", true, 1.0);

        assert_eq!(
            app.compare_jobs_for_sort(&job_a, &job_b),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_jobs_for_sort_by_status() {
        let mut app = App::new(None, ConnectionContext::default());
        app.sort_state.column = SortColumn::Status;

        let job_done = create_test_job("job1", true, 1.0);
        let job_running = create_test_job("job2", false, 1.0);

        // Done jobs come before running jobs in ascending order
        assert_eq!(
            app.compare_jobs_for_sort(&job_done, &job_running),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            app.compare_jobs_for_sort(&job_running, &job_done),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_jobs_for_sort_by_duration_desc() {
        let mut app = App::new(None, ConnectionContext::default());
        app.sort_state.column = SortColumn::Duration;
        app.sort_state.direction = SortDirection::Desc;

        let job_short = create_test_job("job1", true, 1.0);
        let job_long = create_test_job("job2", true, 10.0);

        // In descending order, longer duration comes first
        assert_eq!(
            app.compare_jobs_for_sort(&job_long, &job_short),
            std::cmp::Ordering::Less
        );
    }
}
