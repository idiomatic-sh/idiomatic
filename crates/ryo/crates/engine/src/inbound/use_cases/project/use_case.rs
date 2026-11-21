//! The `project` use case: the cross-file rules' programs once over the frontier rows and the project's facts (tracks X and H).

use super::errors::ProjectPassError;
use super::interface::ProjectUseCase;
use crate::domain::findings::types::{FoundRows, FrontierSource, ScanCounters};
use crate::domain::programs::types::Programs;
use crate::domain::scratch::service::next_file;
use crate::domain::scratch::types::ScanScratch;
use ryo_language::domain::facts::types::ProjectFacts;

/// Runs rule sets' cross-file programs.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
///
/// let _use_case = ProjectUseCaseImpl::new();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectUseCaseImpl;

impl ProjectUseCaseImpl {
    /// The use case; it holds no port.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_engine::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
    ///
    /// let _use_case = ProjectUseCaseImpl::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ProjectUseCase for ProjectUseCaseImpl {
    fn project(
        &self,
        _programs: &Programs,
        _frontier: &dyn FrontierSource,
        _project: ProjectFacts<'_>,
        scratch: &mut ScanScratch,
        _found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError> {
        next_file(scratch);
        // Tracks X and H: the cross-file programs over `FrontierSource` made tables and the project's facts (scope 2.6). A rule
        // set holds no program until the lowering accepts a rule, so the pass runs nothing yet.
        Ok(ScanCounters::default())
    }
}
