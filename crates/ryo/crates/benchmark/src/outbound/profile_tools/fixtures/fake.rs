//! Profilers standing in for the Linux tools: the dumps and totals they answer are given, and what they write is kept in memory.
use crate::outbound::profile_tools::interface::ProfileToolsPort;
use crate::outbound::profile_tools::types::{
    DumpTotal, FileContents, HeapBytes, HeapDumps, ProfileReceipt, SamplingRate,
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The argument the fake's CPU profiler puts before the rate.
const PROFILED_AT: &str = "profiled-at";

/// Deterministic profilers for orchestration and contract tests.
#[derive(Clone, Debug)]
pub struct ProfileToolsFake {
    /// The threads answered.
    pub threads: NonZeroUsize,
    /// The dumps every prepared directory holds, by file name, and their totals.
    pub totals: Vec<DumpTotal>,
    /// The directories prepared.
    pub prepared: Arc<Mutex<Vec<PathBuf>>>,
    /// Every file written, the reports and the receipt among them.
    pub files: Arc<Mutex<BTreeMap<PathBuf, FileContents>>>,
    /// The recordings the CPU profiler was asked to make.
    pub recordings: Arc<Mutex<Vec<PathBuf>>>,
}
impl Default for ProfileToolsFake {
    fn default() -> Self {
        Self {
            threads: NonZeroUsize::MIN,
            totals: Vec::new(),
            prepared: Arc::default(),
            files: Arc::default(),
            recordings: Arc::default(),
        }
    }
}
impl ProfileToolsFake {
    /// Keep `contents` as the file `path`.
    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn keep(&self, path: PathBuf, contents: FileContents) {
        drop(
            self.files
                .lock()
                .expect("fixture files")
                .insert(path, contents),
        );
    }
}
impl ProfileToolsPort for ProfileToolsFake {
    fn threads(&self) -> NonZeroUsize {
        self.threads
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn prepare(&self, directory: &Path) -> Result<(), Error> {
        self.prepared
            .lock()
            .expect("fixture directories")
            .push(directory.into());
        Ok(())
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn dumps(&self, directory: &Path) -> Result<HeapDumps, Error> {
        if !self
            .prepared
            .lock()
            .expect("fixture directories")
            .iter()
            .any(|prepared| prepared == directory)
        {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        let written = self
            .files
            .lock()
            .expect("fixture files")
            .keys()
            .filter(|file| {
                file.parent() == Some(directory)
                    && file
                        .extension()
                        .is_some_and(|extension| extension == "heap")
            })
            .cloned()
            .collect::<Vec<_>>();
        Ok(HeapDumps(
            self.totals
                .iter()
                .map(|total| directory.join(&total.dump))
                .chain(written)
                .collect(),
        ))
    }

    fn write(&self, file: &Path, contents: &FileContents) -> Result<(), Error> {
        self.keep(file.into(), contents.clone());
        Ok(())
    }

    fn heap_total(&self, _worker: &Path, dump: &Path) -> Result<HeapBytes, Error> {
        self.totals
            .iter()
            .find(|total| dump.file_name() == Some(total.dump.as_os_str()))
            .map(|total| total.bytes)
            .ok_or_else(|| Error::Invalid(format!("no total for {}", dump.display())))
    }

    fn heap_report(&self, worker: &Path, dump: &Path, report: &Path) -> Result<(), Error> {
        let _total = self.heap_total(worker, dump)?;
        self.keep(
            report.into(),
            FileContents(dump.as_os_str().as_encoded_bytes().to_vec()),
        );
        Ok(())
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn cpu_invocation(
        &self,
        worker: Invocation,
        rate: SamplingRate,
        recording: &Path,
    ) -> Invocation {
        self.recordings
            .lock()
            .expect("fixture recordings")
            .push(recording.into());
        let mut arguments: Vec<OsString> = vec![
            PROFILED_AT.into(),
            rate.0.to_string().into(),
            recording.into(),
        ];
        arguments.extend(worker.arguments);
        Invocation {
            executable: worker.executable,
            arguments,
            environment: worker.environment,
            directory: worker.directory,
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn cpu_report(&self, recording: &Path, reports: &Path) -> Result<(), Error> {
        if !self
            .recordings
            .lock()
            .expect("fixture recordings")
            .iter()
            .any(|asked| asked == recording)
        {
            return Err(Error::Invalid(format!(
                "no recording at {}",
                recording.display()
            )));
        }
        self.keep(
            reports.join("self.txt"),
            FileContents(recording.as_os_str().as_encoded_bytes().to_vec()),
        );
        Ok(())
    }

    fn receipt(&self, output: &Path, receipt: &ProfileReceipt) -> Result<(), Error> {
        self.keep(
            output.join("receipt.json"),
            FileContents(format!("{receipt:?}").into_bytes()),
        );
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
