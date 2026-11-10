//! The arguments that choose which cases a verification run covers and where it records them.
use super::types::{CaseArgument, CaseRun};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Turn the verification flags of a command line into the typed choices of one run.
pub trait SelectCasesUseCase {
    /// Parse each flag once; a repeated, empty or unknown flag is refused.
    ///
    /// # Errors
    /// Refuses malformed or repeated flags.
    fn select_cases(&self, arguments: &[CaseArgument]) -> Result<CaseRun, ReportsError>;
}
