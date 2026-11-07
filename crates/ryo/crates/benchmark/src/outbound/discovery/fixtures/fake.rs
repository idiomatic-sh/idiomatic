//! A fixed selection standing in for a tool's own discovery.
use crate::outbound::discovery::interface::DiscoveryPort;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// Deterministic discovery substitute for orchestration and contract tests.
#[derive(Clone, Debug, Default)]
pub struct DiscoveryFake {
    /// The files every worktree is answered with.
    pub files: Vec<RelativePath>,
}
impl DiscoveryPort for DiscoveryFake {
    fn files(&self, _root: &Path) -> Result<Vec<RelativePath>, Error> {
        if self.files.is_empty() {
            return Err(Error::Invalid("empty discovered corpus".into()));
        }
        let mut files = self.files.clone();
        files.sort();
        Ok(files)
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
