//! Seals the executables cargo builds and runs them, streaming their output.
use super::constants::{
    BENCH, BUILD_FAILED, CARGO_TOML, EXECUTABLE_MODE, LOCKED, MANIFEST_PATH, MESSAGE_FORMAT,
    NO_OUTPUT, NO_RUN, OPERATIONS_RUN_ON_UNIX, SEAL_SUFFIX, TOOL_NOT_FOUND,
};
use super::settings::ArtifactsCargoSettings;
use crate::domain::operations::types::BenchDirectory;
use crate::inbound::use_cases::operate::types::OperationVerdict;
use crate::outbound::artifacts::interface::ArtifactsPort;
use crate::outbound::artifacts::types::Sealed;
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};
use std::ffi::{OsStr, OsString};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Refuse a platform where `cargo run` waits on its child and keeps the build lock a nested build needs.
pub(super) fn refusal(is_unix: bool) -> Result<(), Error> {
    if is_unix {
        Ok(())
    } else {
        Err(Error::Invalid(OPERATIONS_RUN_ON_UNIX.into()))
    }
}

/// The first file of this name in the directories of the search path, canonicalized.
pub(super) fn find(search_path: &OsStr, tool: &ToolName) -> Result<PathBuf, Error> {
    std::env::split_paths(search_path)
        .map(|directory| directory.join(&tool.0))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| Error::Invalid(format!("{TOOL_NOT_FOUND}: {tool}")))?
        .canonicalize()
        .map_err(Error::from)
}

/// Remove whatever the directory of `sealed` holds and make it again, empty.
fn fresh(sealed: &Path) -> Result<(), Error> {
    if let Some(directory) = sealed.parent() {
        if directory.exists() {
            std::fs::remove_dir_all(directory)?;
        }
        std::fs::create_dir_all(directory)?;
    }
    Ok(())
}

/// Copy `source` to `sealed` as an executable, write its seal record beside it, and answer its digest.
fn seal(source: &Path, sealed: &Path) -> Result<Sealed, Error> {
    let bytes = std::fs::read(source)?;
    std::fs::write(sealed, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(sealed, std::fs::Permissions::from_mode(EXECUTABLE_MODE))?;
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let mut record = sealed.as_os_str().to_owned();
    record.push(SEAL_SUFFIX);
    std::fs::write(record, super::schemas::seal_record(source, &sha256)?)?;
    Ok(Sealed {
        path: sealed.to_owned(),
        sha256: Sha256Text(sha256),
    })
}

/// Cargo builds the worker; the search path finds the reference tool.
#[derive(Debug)]
pub(crate) struct ArtifactsCargoAdapter {
    /// The build tool and the search path.
    settings: ArtifactsCargoSettings,
}

impl ArtifactsCargoAdapter {
    /// The adapter over the build tool and search path the edge read.
    pub(crate) fn new(settings: ArtifactsCargoSettings) -> Self {
        Self { settings }
    }
}

impl ArtifactsPort for ArtifactsCargoAdapter {
    fn target(&self) -> Result<PathBuf, Error> {
        super::schemas::target_directory(&self.settings.cargo)
    }

    fn reference(&self, tool: &ToolName) -> Result<PathBuf, Error> {
        find(&self.settings.search_path, tool)
    }

    fn seal_worker(&self, bench: &BenchDirectory, sealed: &Path) -> Result<Sealed, Error> {
        refusal(cfg!(unix))?;
        fresh(sealed)?;
        let manifest = bench.0.join(CARGO_TOML).canonicalize()?;
        let build = Command::new(&self.settings.cargo)
            .args([BENCH, NO_RUN, LOCKED, MESSAGE_FORMAT, MANIFEST_PATH])
            .arg(&manifest)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()?;
        if !build.status.success() {
            return Err(Error::Invalid(BUILD_FAILED.into()));
        }
        seal(
            &super::schemas::bench_executable(&build.stdout, &manifest)?,
            sealed,
        )
    }

    fn seal_current(&self, sealed: &Path) -> Result<Sealed, Error> {
        seal(&std::env::current_exe()?, sealed)
    }

    fn run_sealed(
        &self,
        executable: &Path,
        arguments: &[OsString],
        output: &mut dyn std::io::Write,
    ) -> Result<OperationVerdict, Error> {
        let mut child = Command::new(executable)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid(NO_OUTPUT.into()))?;
        let mut chunk = [0_u8; 8192];
        loop {
            let read = stdout.read(&mut chunk)?;
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                break;
            };
            output.write_all(bytes)?;
            output.flush()?;
        }
        Ok(if child.wait()?.success() {
            OperationVerdict::Passed
        } else {
            OperationVerdict::Failed
        })
    }
}
