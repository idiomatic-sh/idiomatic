//! The `scaffold` use case's failures.

use crate::domain::placement::errors::PlacementError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::templates::errors::TemplatesError;

/// Why `scaffold` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScaffoldError {
    /// The requested directory is absolute or escapes the repository.
    #[error("scaffold target `{path}` escapes the repository root")]
    InvalidPath {
        /// The requested path.
        path: String,
    },
    /// A condition could not be evaluated.
    #[error(transparent)]
    Analysis(#[from] crate::outbound::analysis::errors::AnalysisError),
    /// A required condition remained unknown.
    #[error("scaffold condition could not be decided: {reason}")]
    Undecided {
        /// The engine's explanation.
        reason: String,
    },
    /// The prospective files do not reach a stable condition and render context.
    #[error("scaffold conditions and template contents do not settle; no files were written")]
    Unsettled,
    /// The repository could not be read or written.
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
    /// The entries could not be placed.
    #[error(transparent)]
    Placement(#[from] PlacementError),
    /// No entry of that name in the workspace's files.
    #[error("no entry `{entry}` in the workspace's tastemaker.toml files")]
    NoEntry {
        /// The entry.
        entry: String,
    },
    /// `--at` is a pattern that still captures, not the directory of one instance.
    #[error("`--at {at}` still captures `{capture}`: name the instance's directory in its place")]
    UnresolvedAt {
        /// The directory asked for.
        at: String,
        /// The capture it still holds.
        capture: String,
    },
    /// `--at` names a directory the spec entry does not place an instance at.
    #[error(
        "`{at}` is not where `{entry}` places an instance: its `include` does not select that directory"
    )]
    OutsideTarget {
        /// The directory asked for.
        at: String,
        /// The entry.
        entry: String,
    },
    /// `--set` names a value nothing the entry places reads.
    #[error("`--set {name}`: `{entry}` reads no capture or param `{name}`")]
    Unread {
        /// The name given.
        name: String,
        /// The entry.
        entry: String,
    },
    /// A file's path still holds a capture no `--set` gives.
    #[error("`{file}` still captures `{capture}`: pass `--set {capture}=<value>`")]
    Capture {
        /// The pattern.
        file: String,
        /// The capture.
        capture: String,
    },
    /// A param the file's template declares has no value.
    #[error("`{file}` needs the param `{param}`: pass `--set {param}=<value>`")]
    Param {
        /// The file.
        file: String,
        /// The param.
        param: String,
    },
    /// The file's template renders nothing: the file is written by the tool that owns it, or by its author.
    #[error(
        "`{file}` is not scaffolded: its template renders nothing, so its tool or its author writes it"
    )]
    Unwritten {
        /// The file.
        file: String,
    },
    /// A template did not render.
    #[error("`{file}`: {source}")]
    Render {
        /// The file.
        file: String,
        /// Why.
        source: TemplatesError,
    },
}
