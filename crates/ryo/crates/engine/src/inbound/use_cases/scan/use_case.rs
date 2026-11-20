//! The `scan` use case: the file's state cleared, then the programs dispatched over the kinds present (track X).

use super::errors::ScanError;
use super::interface::ScanUseCase;
use crate::domain::findings::types::{FoundRows, ScanCounters};
use crate::domain::programs::types::Programs;
use crate::domain::scratch::service::clear_file;
use crate::domain::scratch::types::ScanScratch;
use ryo_language::domain::facts::types::FileFacts;

/// Runs rule sets' programs on files.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
///
/// let _use_case = ScanUseCaseImpl::new();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ScanUseCaseImpl;

impl ScanUseCaseImpl {
    /// The use case; it holds no port.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
    ///
    /// let _use_case = ScanUseCaseImpl::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ScanUseCase for ScanUseCaseImpl {
    fn scan(
        &self,
        _programs: &Programs,
        _file: FileFacts<'_>,
        scratch: &mut ScanScratch,
        _found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError> {
        clear_file(scratch);
        // Track X: dispatch over `KindsPresent`, each anchor bucket's trie once, nodes outer and rules inner (scope 2.6). A rule
        // set holds no program until the lowering (track L1) accepts a rule, so a scan runs nothing yet.
        Ok(ScanCounters::default())
    }
}
