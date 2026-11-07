//! The `packages` port's failures.

/// Why a package could not be found or read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PackagesError {
    /// The source is one this build does not resolve: git and the registry wait for the registry work.
    #[error(
        "package `{package}` comes from {source_kind}, which Tastemaker does not resolve yet: give it a `path`"
    )]
    Unsupported {
        /// The package.
        package: String,
        /// What its source is.
        source_kind: String,
    },
    /// The package's directory is not there.
    #[error("package `{package}`: no directory at `{path}`")]
    Missing {
        /// The package.
        package: String,
        /// Where it was looked for.
        path: String,
    },
    /// A file of a package could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Read {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
}
