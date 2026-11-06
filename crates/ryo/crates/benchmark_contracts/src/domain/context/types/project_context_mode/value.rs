//! Explicit source isolation and stable process spelling.
/// Source isolation contract at the benchmark boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectContextMode {
    /// Given text only; the root supplies no disk context.
    #[default]
    Isolated,
    /// Listed files and exact preloaded bytes, with a sealed disk dependency context.
    ListedDisk,
}

impl ProjectContextMode {
    /// Stable wire spelling included in invocation and certificate identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Isolated => crate::domain::context::types::project_context_mode::constants::ISOLATED,
            Self::ListedDisk => crate::domain::context::types::project_context_mode::constants::LISTED_DISK,
        }
    }
}
