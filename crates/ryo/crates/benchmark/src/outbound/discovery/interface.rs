//! The files a tool would check by default, asked before either timer starts.
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// A tool's own selection of the source files of a worktree.
pub trait DiscoveryPort {
    /// List the files the tool selects under `root`, relative to it and sorted.
    ///
    /// # Errors
    /// Refuses a failed discovery, a file outside the root, a path that is not text and an empty selection.
    fn files(&self, root: &Path) -> Result<Vec<RelativePath>, Error>;
}
