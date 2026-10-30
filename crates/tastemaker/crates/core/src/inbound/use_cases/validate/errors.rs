//! The `validate` use case's failures.

use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `validate` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidateError {
    /// Independent work could not be scheduled.
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
    /// The repository could not be read.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// A definition could not be read.
    #[error("`{file}`: {source}")]
    Manifest {
        /// The file.
        file: String,
        /// Why.
        source: ManifestError,
    },
    /// A package could not be found or read.
    #[error(transparent)]
    Packages(#[from] PackagesError),
    /// The nearest `tastemaker.toml` is a local one.
    #[error(
        "`{file}` is a local tastemaker.toml: run from the workspace whose root file names it in `members`"
    )]
    NotARoot {
        /// The file.
        file: String,
    },
    /// A package holds a file that belongs to nothing in it.
    #[error(
        "`{file}` in package `{package}` is no `package.toml`, nested package or file of a `<kind>/<name>/<version>/` directory"
    )]
    Stray {
        /// The package.
        package: String,
        /// The file, relative to the package's directory.
        file: String,
    },
    /// Versions of the workspace's own packages that no lock of the repository reads and that are not their definition's
    /// latest: each is there for no one.
    #[error(
        "{}: no lock in the repository reads these versions, and none is its definition's latest",
        quoted(.versions)
    )]
    Unread {
        /// The versions, `package/kind/name@version` each.
        versions: Vec<String>,
    },
    /// An explicit selection did not name any available version.
    #[error("validation selection matched no definition versions")]
    NoMatch,
    /// The engine failed.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
}

impl From<crate::application::reporting::errors::ReportingError> for ValidateError {
    fn from(error: crate::application::reporting::errors::ReportingError) -> Self {
        match error {
            crate::application::reporting::errors::ReportingError::Scheduler(error) => {
                Self::Scheduler(error)
            }
        }
    }
}

/// `versions`, each in backticks, comma separated: as a message names a definition.
fn quoted(versions: &[String]) -> String {
    versions
        .iter()
        .map(|version| format!("`{version}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
