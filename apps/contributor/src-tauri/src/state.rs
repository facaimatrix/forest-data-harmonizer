use std::sync::Mutex;

use gfb3_core::mapping::ContributorMapping;
use gfb3_core::validation::ValidationReport;
use polars::prelude::DataFrame;

/// In-memory state for the current wizard session.
/// One session = one loaded file moving through the 6-step guided flow.
pub struct SessionState {
    pub raw_df: DataFrame,
    pub file_path: String,
    /// Set after the contributor completes the column-mapping step.
    pub mapped_df: Option<DataFrame>,
    pub mapping: Option<ContributorMapping>,
    /// Set after the validation step is run.
    pub validation_report: Option<ValidationReport>,
}

/// Shared application state held by Tauri across IPC calls.
#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<SessionState>>,
}
