//! A scripted executables boundary that records what an operation asks of it.
use crate::outbound::executables::interface::ExecutablesPort;
use crate::outbound::executables::types::{BenchPackage, RunArguments, RunVerdict, Sealed};
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One request the fake answered, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutablesCall {
    /// The bench's worker was sealed at this path.
    SealWorker(PathBuf),
    /// The running executable was sealed at this path.
    SealCurrent(PathBuf),
    /// A sealed executable ran with these arguments.
    Run(PathBuf, Vec<OsString>),
}

/// Deterministic executables: a fixed target, the tools it knows, and what a sealed run writes and answers.
#[derive(Debug)]
pub struct ExecutablesFake {
    /// The target directory every operation writes under.
    pub target: PathBuf,
    /// The reference tools on the fake search path, by name.
    pub tools: Vec<(ToolName, PathBuf)>,
    /// What every sealed run writes to its output.
    pub stdout: Vec<u8>,
    /// What every sealed run answers.
    pub verdict: RunVerdict,
    /// Every request, in order.
    pub history: Mutex<Vec<ExecutablesCall>>,
}

impl ExecutablesFake {
    /// Record one request.
    fn record(&self, call: ExecutablesCall) {
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

impl ExecutablesPort for ExecutablesFake {
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

    fn seal_worker(&self, bench: &BenchPackage, sealed: &Path) -> Result<Sealed, Error> {
        let _bench = std::fs::metadata(&bench.0)?;
        self.record(ExecutablesCall::SealWorker(sealed.to_owned()));
        if let Some(directory) = sealed.parent() {
            match std::fs::remove_dir_all(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            std::fs::create_dir_all(directory)?;
        }
        seal(sealed, "worker")
    }

    fn seal_current(&self, sealed: &Path) -> Result<Sealed, Error> {
        self.record(ExecutablesCall::SealCurrent(sealed.to_owned()));
        seal(sealed, "collector")
    }

    fn run_sealed(
        &self,
        executable: &Path,
        arguments: &RunArguments,
        output: &mut dyn std::io::Write,
    ) -> Result<RunVerdict, Error> {
        let _executable = std::fs::metadata(executable)?;
        self.record(ExecutablesCall::Run(executable.to_owned(), arguments.0.clone()));
        output.write_all(&self.stdout)?;
        Ok(self.verdict)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
