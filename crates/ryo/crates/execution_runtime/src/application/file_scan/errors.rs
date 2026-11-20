//! Why the file scan keeps no complete rows of a file.

use ryo_engine::domain::findings::types::FoundTarget;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::inbound::use_cases::walk::errors::WalkError;

/// Why a file's scan is a fault of the file, or of a rule in it: the run goes on and reports it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::errors::FileScanError;
/// use ryo_language::inbound::use_cases::walk::errors::WalkError;
///
/// let fault = FileScanError::Walk { cause: WalkError::Unsupported };
/// assert_eq!(fault.to_string(), "file not walked: the language does not read this file");
/// ```
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileScanError {
    /// The language did not walk the file: it does not read it, or its parser failed.
    #[error("file not walked: {cause}")]
    Walk {
        /// Why the language refused.
        cause: WalkError,
    },
    /// The language walked the file and lent it to no visit.
    #[error("file walked and not lent")]
    NotLent,
    /// A rule's program read a relation the language refused or filled in part: the rule's rows of the file are not complete.
    #[error("{fault}")]
    Rule {
        /// The engine's fault, naming the rule and the relation.
        fault: ScanError,
    },
    /// A found row does not fit the columns the host holds for its rule or frontier entry.
    #[error("found row of {target:?} does not fit its columns")]
    Columns {
        /// Whose row it is.
        target: FoundTarget,
    },
}
