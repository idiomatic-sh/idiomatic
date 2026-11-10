//! The listing adapter runs the tool's command in the worktree and reads one path per line.
use super::constants::{DISCOVERED_OUTSIDE, EMPTY_DISCOVERY, NON_UTF8_PATH};
use crate::outbound::discovery::interface::DiscoveryPort;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Asks a tool's own command which files it selects.
#[derive(Debug)]
pub(crate) struct DiscoveryListingAdapter {
    /// The pinned executable of the tool.
    executable: PathBuf,
    /// The arguments that make it list the files it selects, one path per line.
    arguments: Vec<OsString>,
}
impl DiscoveryListingAdapter {
    /// Bind the command that lists a worktree's files.
    pub(crate) fn new(executable: PathBuf, arguments: Vec<OsString>) -> Self {
        Self {
            executable,
            arguments,
        }
    }
}
impl DiscoveryPort for DiscoveryListingAdapter {
    fn files(&self, root: &Path) -> Result<Vec<RelativePath>, Error> {
        let output = std::process::Command::new(&self.executable)
            .args(&self.arguments)
            .current_dir(root)
            .output()?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err(Error::Invalid(format!(
                "file discovery failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let text = std::str::from_utf8(&output.stdout)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let mut files = Vec::new();
        for line in text.lines() {
            let path = Path::new(line)
                .strip_prefix(root)
                .map_err(|_outside_root| Error::Invalid(DISCOVERED_OUTSIDE.into()))?;
            let file = path
                .to_str()
                .ok_or_else(|| Error::Invalid(NON_UTF8_PATH.into()))?;
            RelativeArtifactPath::try_from(Path::new(file))?;
            files.push(RelativePath(file.into()));
        }
        files.sort();
        if files.is_empty() {
            return Err(Error::Invalid(EMPTY_DISCOVERY.into()));
        }
        Ok(files)
    }
}
