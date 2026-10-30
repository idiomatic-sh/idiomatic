//! The `manifest` port's failures.

/// Why the product's `asymptote.toml` could not be found or read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// The workspace root pins another asymptote than the one that runs.
    #[error("this repo needs asymptote {needed}; {installed} is installed")]
    Pinned {
        /// The version the root pins.
        needed: String,
        /// The version that runs.
        installed: String,
    },
    /// No directory at or above the start holds one.
    #[error("no asymptote.toml at or above `{start}`")]
    Missing {
        /// Where the search started.
        start: String,
    },
    /// It could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
    /// It is not the shape asymptote reads.
    #[error("`{path}`: {reason}")]
    Shape {
        /// The file.
        path: String,
        /// What is wrong.
        reason: String,
    },
    /// A member of the workspace cannot be a member.
    #[error("workspace member `{member}`: {reason}")]
    Member {
        /// The member, as the workspace names it or as it was found.
        member: String,
        /// What is wrong.
        reason: String,
    },
    /// `members` is declared where a workspace cannot be.
    #[error("`{path}` declares `members`, but an asymptote.toml above it is the workspace root")]
    NotRoot {
        /// The file.
        path: String,
    },
    /// A `[hooks]` entry cannot be run.
    #[error("hook `{hook}`: {reason}")]
    Hook {
        /// The hook.
        hook: String,
        /// What is wrong.
        reason: String,
    },
    /// `asymptote.toml` could not be written.
    #[error("cannot write `{path}`: {reason}")]
    Unwritable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
}
