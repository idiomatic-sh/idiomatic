//! Failures while checking whether the retained report is current.

/// Why a retained report cannot authorize a ratchet operation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CachedReportError {
    /// No check ran under this build.
    #[error("no check has run here with this asymptote")]
    NoReport,
    /// The current commit differs from the report.
    #[error("the working tree is on another commit than the last check ran on")]
    HeadMoved,
    /// The tool configuration changed.
    #[error("asymptote.toml changed since the last check")]
    ManifestMoved,
    /// A source file changed after the check.
    #[error("{count} file(s) changed since the last check, `{first}` among them")]
    Moved {
        /// The first changed file.
        first: String,
        /// The number of changed files.
        count: usize,
    },
    /// The report contains incomplete tool results.
    #[error("{count} tool(s) failed in the last check, `{first}` among them")]
    ToolsFailed {
        /// The first failed tool.
        first: String,
        /// The number of failed tools.
        count: usize,
    },
}
