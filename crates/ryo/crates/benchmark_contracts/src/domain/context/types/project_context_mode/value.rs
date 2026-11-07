//! Explicit source isolation.
/// Source isolation contract at the benchmark boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectContextMode {
    /// Given text only; the root supplies no disk context.
    #[default]
    Isolated,
    /// Listed files and exact preloaded bytes, with a sealed disk dependency context.
    ListedDisk,
}
