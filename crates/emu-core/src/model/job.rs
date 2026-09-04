//! [`Job`] — a tracked long-running operation (download, create, launch, …).
//!
//! The orchestrator (milestone M1+) owns jobs; `emu-core` here only defines their shape and the
//! [`JobHandle`] progress channel the ports write to.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Identifier for a job (a ULID string in practice).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub String);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a job is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum JobKind {
    /// A single file download.
    Download,
    /// Ensure a system image is installed (may fan out to downloads + `sdkmanager`).
    EnsureImage,
    /// Create an AVD.
    Create,
    /// Launch an emulator and wait for boot.
    Launch,
    /// Apply a profile end to end.
    ApplyProfile,
    /// Update SDK components.
    Update,
}

/// Lifecycle of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum JobState {
    /// Waiting to start.
    Queued,
    /// In progress.
    Running,
    /// Finished successfully.
    Done,
    /// Finished with an error (see [`Job::error`]).
    Failed,
    /// Cancelled by the user.
    Cancelled,
}

/// A unit of tracked work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    /// Job id.
    pub id: JobId,
    /// What kind of work this is.
    pub kind: JobKind,
    /// Current state.
    pub state: JobState,
    /// Short human phase label, e.g. `"Downloading system image (2 of 3)"`.
    pub phase: String,
    /// Progress percentage in `0..=100`, or `None` when indeterminate.
    pub pct: Option<u8>,
    /// Estimated seconds remaining, when known.
    pub eta_secs: Option<u64>,
    /// When the job started running.
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    /// When the job reached a terminal state.
    #[serde(with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
    /// Error message when `state == Failed`.
    pub error: Option<String>,
}

/// A progress update pushed by a port while a job runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// New phase label, if it changed.
    pub phase: Option<String>,
    /// Bytes completed so far (for downloads).
    pub bytes_done: Option<u64>,
    /// Total bytes expected, when known.
    pub bytes_total: Option<u64>,
    /// Percentage `0..=100`, when computable.
    pub pct: Option<u8>,
    /// A single log line to append to the job's tail.
    pub log_line: Option<String>,
}

impl Progress {
    /// An empty update (no fields set).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            phase: None,
            bytes_done: None,
            bytes_total: None,
            pct: None,
            log_line: None,
        }
    }

    /// A pure log-line update.
    #[must_use]
    pub fn log(line: impl Into<String>) -> Self {
        Self {
            log_line: Some(line.into()),
            ..Self::empty()
        }
    }
}

/// Sink a port uses to report [`Progress`] for the job it was handed.
///
/// Deliberately a plain callback wrapper so `emu-core` needs no async-runtime dependency. The
/// orchestrator supplies one backed by an event channel; tests supply [`JobHandle::collector`].
pub struct JobHandle {
    id: JobId,
    sink: Box<dyn Fn(Progress) + Send + Sync>,
}

impl JobHandle {
    /// Build a handle from a job id and a progress callback.
    #[must_use]
    pub fn new(id: JobId, sink: Box<dyn Fn(Progress) + Send + Sync>) -> Self {
        Self { id, sink }
    }

    /// A handle that drops every update — for code paths that don't care.
    #[must_use]
    pub fn noop(id: JobId) -> Self {
        Self {
            id,
            sink: Box::new(|_| {}),
        }
    }

    /// The job this handle reports to.
    #[must_use]
    pub fn id(&self) -> &JobId {
        &self.id
    }

    /// Emit a progress update.
    pub fn report(&self, p: Progress) {
        (self.sink)(p);
    }

    /// A handle plus a shared vec that captures everything reported. Test helper.
    #[must_use]
    pub fn collector(id: JobId) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<Progress>>>) {
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_log = std::sync::Arc::clone(&log);
        let handle = Self::new(
            id,
            Box::new(move |p| sink_log.lock().expect("progress log poisoned").push(p)),
        );
        (handle, log)
    }
}

impl std::fmt::Debug for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_json_round_trips_with_optional_times() {
        let job = Job {
            id: JobId("01J00000000000000000000JOB".into()),
            kind: JobKind::EnsureImage,
            state: JobState::Running,
            phase: "Downloading (1 of 2)".into(),
            pct: Some(42),
            eta_secs: Some(90),
            started_at: Some(time::macros::datetime!(2026-09-05 10:00:00 UTC)),
            finished_at: None,
            error: None,
        };
        let json = serde_json::to_string(&job).expect("ser");
        let back: Job = serde_json::from_str(&json).expect("de");
        assert_eq!(job, back);
        assert!(json.contains("\"finishedAt\":null"));
    }

    #[test]
    fn job_handle_reports_through_collector() {
        let (handle, log) = JobHandle::collector(JobId("j1".into()));
        assert_eq!(handle.id().0, "j1");
        handle.report(Progress::log("hello"));
        handle.report(Progress {
            pct: Some(50),
            ..Progress::empty()
        });
        let got = log.lock().expect("lock");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].log_line.as_deref(), Some("hello"));
        assert_eq!(got[1].pct, Some(50));
    }

    #[test]
    fn noop_handle_swallows_updates() {
        let h = JobHandle::noop(JobId("j2".into()));
        h.report(Progress::log("ignored")); // must not panic
    }
}
