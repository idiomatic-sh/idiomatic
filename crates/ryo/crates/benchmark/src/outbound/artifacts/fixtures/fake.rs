//! A scripted artifacts boundary that records what an operation asks of it.
use crate::domain::operations::types::BenchDirectory;
use crate::inbound::use_cases::operate::types::OperationVerdict;
use crate::outbound::artifacts::interface::ArtifactsPort;
use crate::outbound::artifacts::types::Sealed;
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One request the fake answered, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactsCall {
    /// The bench's worker was sealed at this path.
    SealWorker(PathBuf),
    /// The running executable was sealed at this path.
    SealCurrent(PathBuf),
    /// A sealed executable ran with these arguments.
    Run(PathBuf, Vec<OsString>),
}

/// Deterministic artifacts: a fixed target, the tools it knows, and what a sealed run writes and answers.
#[derive(Debug)]
pub struct ArtifactsFake {
    /// The target directory every operation writes under.
    pub target: PathBuf,
    /// The reference tools on the fake search path, by name.
    pub tools: Vec<(ToolName, PathBuf)>,
    /// What every sealed run writes to its output.
    pub stdout: Vec<u8>,
    /// What every sealed run answers.
    pub verdict: OperationVerdict,
    /// Every request, in order.
    pub history: Mutex<Vec<ArtifactsCall>>,
}

impl ArtifactsFake {
    /// Record one request.
    fn record(&self, call: ArtifactsCall) {
        if let Ok(mut history) = self.history.lock() {
            history.push(call);
        }
    }
}

/// Write a stand-in executable and answer its seal.
fn seal(sealed: &Path, contents: &str) -> Result<Sealed, Error> {
    std::fs::write(sealed, contents)?;
    Ok(Sealed {
        path: sealed.to_owned(),
        sha256: Sha256Text(format!("{:x}", Sha256::digest(contents))),
    })
}

impl ArtifactsPort for ArtifactsFake {
    fn target(&self) -> Result<PathBuf, Error> {
        Ok(self.target.clone())
    }

    fn reference(&self, tool: &ToolName) -> Result<PathBuf, Error> {
        self.tools
            .iter()
            .find(|(name, _path)| name == tool)
            .map(|(_name, path)| path.clone())
            .ok_or_else(|| Error::Invalid(format!("reference tool not found: {tool}")))
    }

    fn seal_worker(&self, _bench: &BenchDirectory, sealed: &Path) -> Result<Sealed, Error> {
        self.record(ArtifactsCall::SealWorker(sealed.to_owned()));
        if let Some(directory) = sealed.parent() {
            if directory.exists() {
                std::fs::remove_dir_all(directory)?;
            }
            std::fs::create_dir_all(directory)?;
        }
        seal(sealed, "worker")
    }

    fn seal_current(&self, sealed: &Path) -> Result<Sealed, Error> {
        self.record(ArtifactsCall::SealCurrent(sealed.to_owned()));
        seal(sealed, "collector")
    }

    fn run_sealed(
        &self,
        executable: &Path,
        arguments: &[OsString],
        output: &mut dyn std::io::Write,
    ) -> Result<OperationVerdict, Error> {
        self.record(ArtifactsCall::Run(executable.to_owned(), arguments.to_vec()));
        output.write_all(&self.stdout)?;
        Ok(self.verdict)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
