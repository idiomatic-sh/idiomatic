//! The `manifest` port's failures.

/// Why a definition's text could not be read or edited.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// The text is not TOML, or not of the format's shape.
    #[error("{reason}")]
    Shape {
        /// What is wrong, and where.
        reason: String,
    },
    /// A lock that does not begin with the version this build writes: `tm check --fix` writes it again.
    #[error("{reason}")]
    LockVersion {
        /// What is wrong, and how it is mended.
        reason: String,
    },
    /// The workspace needs another version of Tastemaker than the one that runs.
    #[error("this repo needs tastemaker {needs}; {installed} is installed")]
    Needs {
        /// The version the root `tastemaker.toml` names.
        needs: String,
        /// The version that runs.
        installed: String,
    },
}
