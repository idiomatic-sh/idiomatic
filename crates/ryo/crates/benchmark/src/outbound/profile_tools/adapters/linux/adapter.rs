//! The Linux adapter runs `jeprof`, `perf` and `c++filt` and keeps a profile run's files on disk.
use super::constants::{
    CHILDREN_REPORT, COMMAND, CXXFILT, DUMP_EXTENSION, HEAP_REPORT, HEAP_TOTAL, JEPROF, NO_REPORT,
    NO_TOTAL, PERF, RECEIPT, RECORD, RECORD_CALLS, RECORD_EVENT, REPORT, SELF_REPORT, TOOL_FAILED,
    TOTAL,
};
use super::schemas::document;
use crate::domain::profiles::types::{HeapBytes, ProfileReceipt, SamplingRate};
use crate::outbound::profile_tools::interface::ProfileToolsPort;
use crate::outbound::profile_tools::types::{FileContents, HeapDumps};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// The Linux profilers, found on the `PATH`.
#[derive(Debug)]
pub(crate) struct ProfileToolsLinuxAdapter;

/// The output of a profiler that exited successfully.
fn succeeded(output: Output) -> Result<Output, Error> {
    if output.status.success() {
        Ok(output)
    } else {
        Err(Error::Invalid(format!(
            "{TOOL_FAILED}: {}; {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// The total `jeprof --text --show_bytes` prints on its `Total: <bytes> B` line.
pub(super) fn total(report: &[u8]) -> Result<HeapBytes, Error> {
    String::from_utf8_lossy(report)
        .lines()
        .find_map(|line| line.trim().strip_prefix(TOTAL))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|bytes| bytes.parse().ok())
        .map(HeapBytes)
        .ok_or_else(|| Error::Invalid(NO_TOTAL.into()))
}

impl ProfileToolsPort for ProfileToolsLinuxAdapter {
    fn threads(&self) -> NonZeroUsize {
        std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
    }

    fn prepare(&self, directory: &Path) -> Result<(), Error> {
        Ok(std::fs::create_dir_all(directory)?)
    }

    fn dumps(&self, directory: &Path) -> Result<HeapDumps, Error> {
        let mut dumps = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == DUMP_EXTENSION)
            {
                dumps.push(path);
            }
        }
        Ok(HeapDumps(dumps))
    }

    fn write(&self, file: &Path, contents: &FileContents) -> Result<(), Error> {
        Ok(std::fs::write(file, &contents.0)?)
    }

    fn heap_total(&self, worker: &Path, dump: &Path) -> Result<HeapBytes, Error> {
        let output = Command::new(JEPROF)
            .args(HEAP_TOTAL)
            .arg(worker)
            .arg(dump)
            .output()?;
        total(&succeeded(output)?.stdout)
    }

    fn heap_report(&self, worker: &Path, dump: &Path, report: &Path) -> Result<(), Error> {
        let output = Command::new(JEPROF)
            .args(HEAP_REPORT)
            .arg(worker)
            .arg(dump)
            .output()?;
        Ok(std::fs::write(report, succeeded(output)?.stdout)?)
    }

    fn cpu_invocation(
        &self,
        worker: Invocation,
        rate: SamplingRate,
        recording: &Path,
    ) -> Invocation {
        let mut arguments: Vec<OsString> = vec![RECORD.into()];
        arguments.extend(RECORD_EVENT.map(OsString::from));
        arguments.push(rate.0.to_string().into());
        arguments.extend(RECORD_CALLS.map(OsString::from));
        arguments.push(recording.into());
        arguments.push(COMMAND.into());
        arguments.push(worker.executable.into_os_string());
        arguments.extend(worker.arguments);
        Invocation {
            executable: PERF.into(),
            arguments,
            environment: worker.environment,
            directory: worker.directory,
        }
    }

    fn cpu_report(&self, recording: &Path, directory: &Path) -> Result<(), Error> {
        for [order, name] in [SELF_REPORT, CHILDREN_REPORT] {
            let mut report = Command::new(PERF)
                .args(REPORT)
                .arg(recording)
                .arg(order)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;
            let lines = report
                .stdout
                .take()
                .ok_or_else(|| Error::Invalid(NO_REPORT.into()))?;
            let file = std::fs::File::create(directory.join(name))?;
            let demangled = Command::new(CXXFILT)
                .stdin(Stdio::from(lines))
                .stdout(Stdio::from(file))
                .output()?;
            drop(succeeded(demangled)?);
            drop(succeeded(report.wait_with_output()?)?);
        }
        Ok(())
    }

    fn receipt(&self, directory: &Path, receipt: &ProfileReceipt) -> Result<(), Error> {
        Ok(std::fs::write(directory.join(RECEIPT), document(receipt)?.0)?)
    }
}
