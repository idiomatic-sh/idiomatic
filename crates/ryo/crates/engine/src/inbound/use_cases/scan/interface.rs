//! What `scan` promises: one file, once; every program whose anchor occurs in the file runs, a relation is filled only when an
//! enabled program first reads it, and each found row is handed over once.

use super::errors::ScanError;
use crate::domain::findings::types::{FoundRows, ScanCounters};
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::ScanScratch;
use ryo_language::domain::facts::types::FileFacts;

/// Runs a rule set's programs on one file.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::scan::interface::ScanUseCase;
/// use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
///
/// fn scanner() -> impl ScanUseCase {
///     ScanUseCaseImpl::new()
/// }
/// let _scanner = scanner();
/// ```
pub trait ScanUseCase: Send + Sync {
    /// Runs every enabled program over the file's node table and facts. Which file of the run it is, is the lent file's own
    /// `file`: a program writes it as a `File` cell and as the file half of a node's id.
    ///
    /// # Errors
    ///
    /// `Refused` or `Partial` when the language refuses or partly fills a relation a candidate needs: a fault of the rules that
    /// read it, never empty rows.
    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError>;
}
