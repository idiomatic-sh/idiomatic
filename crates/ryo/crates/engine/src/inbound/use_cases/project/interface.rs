//! What `project` promises: once per run, after every file, the cross-file rules' programs over their frontier rows and the
//! project's facts.

use super::errors::ProjectPassError;
use crate::domain::findings::types::{FoundRows, FrontierSource, ScanCounters};
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::ScanScratch;
use ryo_language::domain::facts::types::ProjectFacts;

/// Runs the cross-file rules once per run.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::project::interface::ProjectUseCase;
/// use ryo_engine::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
///
/// fn pass() -> impl ProjectUseCase {
///     ProjectUseCaseImpl::new()
/// }
/// let _pass = pass();
/// ```
pub trait ProjectUseCase: Send + Sync {
    /// Runs the project programs; their made tables are filled from `frontier` and `project`.
    ///
    /// # Errors
    ///
    /// `Refused` when the project refuses a relation a program needs.
    fn project(
        &self,
        programs: &Programs,
        frontier: &dyn FrontierSource,
        project: ProjectFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError>;
}
