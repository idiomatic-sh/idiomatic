//! The fake of the `inventory` port: what the core's tests stand in for it with. It remembers
//! which outputs were published and which observation directories were captured.
use super::super::interface::InventoryPort;
use super::super::types::{
    ContextFiles, InvocationDocument, RawArtifacts, ReceiptFile, Registration, Row, Summary,
};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What the fake remembers between calls.
#[derive(Debug, Default)]
struct Remembered {
    /// Outputs a summary was published to.
    published: BTreeSet<PathBuf>,
    /// Observation directories whose streams were captured.
    captured: BTreeSet<PathBuf>,
}

/// What the core's tests stand in for `inventory` with.
#[derive(Debug, Default)]
pub struct InventoryFake {
    remembered: Mutex<Remembered>,
}

impl InventoryFake {
    /// What the fake remembers, even after a panic in another test thread.
    fn state(&self) -> std::sync::MutexGuard<'_, Remembered> {
        self.remembered
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl InventoryPort for InventoryFake {
    fn load(&self, path: &Path) -> Result<Registration, Error> {
        Err(Error::Invalid(format!("no registration at {}", path.display())))
    }
    fn begin(&self, output: &Path) -> Result<(), Error> {
        if self.state().published.contains(output) {
            return Err(Error::Invalid("output already contains a summary".into()));
        }
        Ok(())
    }
    fn publish(&self, output: &Path, _summary: &Summary) -> Result<(), Error> {
        let _new = self.state().published.insert(output.to_owned());
        Ok(())
    }
    fn prepare(&self, observations: &Path) -> Result<(), Error> {
        if self.state().captured.contains(observations) {
            return Err(Error::Invalid("rule output already contains receipts".into()));
        }
        Ok(())
    }
    fn capture(
        &self,
        observations: &Path,
        _stdout: &CapturedBytes,
        _stderr: &CapturedBytes,
    ) -> Result<ReceiptFile, Error> {
        let _new = self.state().captured.insert(observations.to_owned());
        Ok(ReceiptFile::Absent)
    }
    fn result(&self, _observations: &Path, _row: &Row) -> Result<(), Error> {
        Ok(())
    }
    fn decode_invocation(&self, _bytes: &CapturedBytes) -> Result<InvocationDocument, Error> {
        Err(Error::Invalid("not an invocation record".into()))
    }
    fn raw_artifacts(
        &self,
        _receipt: &Path,
        _records: &[ObservationDocument],
    ) -> Result<RawArtifacts, Error> {
        Ok(RawArtifacts {
            bytes: BTreeMap::new(),
            invocations: BTreeMap::new(),
        })
    }
    fn context_files(&self, context: &Path) -> Result<ContextFiles, Error> {
        Err(Error::Invalid(format!("no context at {}", context.display())))
    }
    fn audit_report(&self, _audit: &ReceiptAudit) -> Result<CapturedBytes, Error> {
        Ok(CapturedBytes(b"{}\n".to_vec()))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
