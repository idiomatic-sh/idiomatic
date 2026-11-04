//! Injected inventory artifact boundary; no domain or application imports.
use super::types::{CapturedObservation, Registration, Row, Summary};
use std::path::Path;

/// Read registrations and persist exact inventory observations.
pub trait InventoryPort {
    /// Read canonical facts without assigning coverage or scheduling policy.
    fn canonical(
        &self,
        authority: &super::types::RegistrationAuthority,
    ) -> Result<super::types::CanonicalFacts, String>;
    /// Recheck every context file; errors retain the exact path.
    fn verify_artifacts(
        &self,
        artifacts: &std::collections::BTreeMap<std::path::PathBuf, String>,
    ) -> Result<(), String>;

    /// Read one explicit receipt file and only its referenced process artifact attempts.
    ///
    /// # Errors
    /// Refuses unsafe paths, missing bytes or malformed observations.
    fn observations(&self, _receipt: &Path) -> Result<CapturedObservation, String> {
        Err("explicit receipt loading is unavailable".into())
    }

    /// Decode the adapter's generated invocation registration.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn load(&self, path: &Path) -> Result<Registration, String>;
    /// Begin a fresh report while retaining already staged reproductions.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn begin(&self, output: &Path) -> Result<(), String>;
    /// Atomically replace the complete summary.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn publish(&self, output: &Path, summary: &Summary) -> Result<(), String>;
    /// Refuse receipt reuse and prepare one rule's observation directory.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn prepare(&self, directory: &Path) -> Result<(), String>;
    /// Keep exact child streams and read any native case observations.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn capture(
        &self,
        directory: &Path,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Result<CapturedObservation, String>;
    /// Keep a terminal row beside its exact native observations.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding, filesystem or observation errors.
    fn result(&self, directory: &Path, row: &Row) -> Result<(), String>;
}
