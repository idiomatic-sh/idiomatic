//! Seals the executables the build tool builds and runs them, publishing only the document a run writes.
use super::constants::{
    BENCH, BUILD_FAILED, CHUNK, LOCKED, MESSAGE_FORMAT, NO_OUTPUT, NO_RUN, OPERATIONS_RUN_ON_UNIX,
    SEAL_SUFFIX, TOOL_NOT_FOUND,
};
use super::settings::ExecutablesCargoSettings;
use super::types::BuildPlatform;
use crate::outbound::executables::interface::ExecutablesPort;
use crate::outbound::executables::types::{BenchPackage, RunArguments, RunVerdict, Sealed};
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};
use std::ffi::OsStr;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The platform this build runs on.
const PLATFORM: BuildPlatform = if cfg!(unix) {
    BuildPlatform::Unix
} else {
    BuildPlatform::Other
};

/// Refuse a platform where the outer build waits on its child and keeps the lock a nested build needs.
pub(super) fn refusal(platform: BuildPlatform) -> Result<(), Error> {
    match platform {
        BuildPlatform::Unix => Ok(()),
        BuildPlatform::Other => Err(Error::Invalid(OPERATIONS_RUN_ON_UNIX.into())),
    }
}

/// The first file of this name in the directories of the search path, canonicalized.
pub(super) fn find(search_path: &OsStr, tool: &ToolName) -> Result<PathBuf, Error> {
    for directory in std::env::split_paths(search_path) {
        if let Ok(path) = directory.join(&tool.0).canonicalize()
            && path.is_file()
        {
            return Ok(path);
        }
    }
    Err(Error::Invalid(format!("{TOOL_NOT_FOUND}: {tool}")))
}

/// Remove whatever the directory of `sealed` holds and make it again, empty.
fn fresh(sealed: &Path) -> Result<(), Error> {
    if let Some(directory) = sealed.parent() {
        match std::fs::remove_dir_all(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        std::fs::create_dir_all(directory)?;
    }
    Ok(())
}

/// Copy `source` to `sealed` and let the copy be executed, through a child (`COPY_EXECUTABLE`).
#[cfg(unix)]
fn copy_executable(source: &Path, sealed: &Path) -> std::io::Result<()> {
    let status = Command::new(super::constants::SHELL)
        .args(super::constants::COPY_EXECUTABLE)
        .arg(source)
        .arg(sealed)
        .stdin(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(status.to_string()))
    }
}

/// Off Unix a copied file keeps whatever lets it run, and a launch never finds it busy.
#[cfg(not(unix))]
fn copy_executable(source: &Path, sealed: &Path) -> std::io::Result<()> {
    std::fs::copy(source, sealed).map(drop)
}

/// Copy `source` to `sealed` as an executable, write its seal record beside it, and answer its digest.
fn seal(source: &Path, sealed: &Path) -> Result<Sealed, Error> {
    let bytes = std::fs::read(source)?;
    copy_executable(source, sealed)?;
    let copy = Sealed {
        path: sealed.to_owned(),
        sha256: Sha256Text(format!("{:x}", Sha256::digest(&bytes))),
    };
    let mut record = sealed.as_os_str().to_owned();
    record.push(SEAL_SUFFIX);
    std::fs::write(record, super::schemas::seal_record(source, &copy)?)?;
    Ok(copy)
}

/// The build tool builds the worker; the search path finds the reference tool.
#[derive(Debug)]
pub(crate) struct ExecutablesCargoAdapter {
    /// The build tool and the search path.
    settings: ExecutablesCargoSettings,
}

impl ExecutablesCargoAdapter {
    /// The adapter over the build tool and search path the edge read.
    pub(crate) fn new(settings: ExecutablesCargoSettings) -> Self {
        Self { settings }
    }
}

impl ExecutablesPort for ExecutablesCargoAdapter {
    fn target(&self) -> Result<PathBuf, Error> {
        super::schemas::target_directory(&self.settings.cargo)
    }

    fn reference(&self, tool: &ToolName) -> Result<PathBuf, Error> {
        find(&self.settings.search_path, tool)
    }

    fn seal_worker(&self, bench: &BenchPackage, sealed: &Path) -> Result<Sealed, Error> {
        refusal(PLATFORM)?;
        let package = bench.0.canonicalize()?;
        fresh(sealed)?;
        let build = Command::new(&self.settings.cargo)
            .args([BENCH, NO_RUN, LOCKED, MESSAGE_FORMAT])
            .current_dir(&package)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()?;
        if !build.status.success() {
            return Err(Error::Invalid(BUILD_FAILED.into()));
        }
        seal(
            &super::schemas::bench_executable(&build.stdout, &package)?,
            sealed,
        )
    }

    fn seal_current(&self, sealed: &Path) -> Result<Sealed, Error> {
        seal(&std::env::current_exe()?, sealed)
    }

    fn check_sealed(
        &self,
        executable: &Path,
        arguments: &RunArguments,
        progress: &mut dyn std::io::Write,
    ) -> Result<RunVerdict, Error> {
        streamed(executable, arguments, progress)
    }

    fn run_sealed(
        &self,
        executable: &Path,
        arguments: &RunArguments,
        document: &Path,
        output: &mut dyn std::io::Write,
        progress: &mut dyn std::io::Write,
    ) -> Result<RunVerdict, Error> {
        if streamed(executable, arguments, progress)? == RunVerdict::Failed {
            return Ok(RunVerdict::Failed);
        }
        output.write_all(&std::fs::read(document)?)?;
        output.flush()?;
        Ok(RunVerdict::Passed)
    }
}

/// Run a sealed executable, streaming its standard output to `progress` as it comes and its errors to this process's
/// error stream, and answer its verdict.
fn streamed(
    executable: &Path,
    arguments: &RunArguments,
    progress: &mut dyn std::io::Write,
) -> Result<RunVerdict, Error> {
    let mut child = Command::new(executable)
        .args(&arguments.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Invalid(NO_OUTPUT.into()))?;
    let mut chunk = [0_u8; CHUNK];
    loop {
        let read = stdout.read(&mut chunk)?;
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            break;
        };
        progress.write_all(bytes)?;
        progress.flush()?;
    }
    Ok(if child.wait()?.success() {
        RunVerdict::Passed
    } else {
        RunVerdict::Failed
    })
}
