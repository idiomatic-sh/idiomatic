//! What the `context` port asks of the outside, in the core's own vocabulary: plain data in and out,
//! and the handles (traits this file declares) it hands back. It never takes an adapter's
//! configuration (a directory, a budget, a client): the port's factory takes that.
use super::types::CorpusFiles;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::CaseContext;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::path::Path;

/// What every `context` adapter implements: sealing, materializing and receipting a project context.
pub trait ContextPort {
    /// Content seal of a manifest: declared bytes, declared absences and the stable logical root,
    /// never the temporary physical directory.
    fn fingerprint(&self, manifest: &ContextManifest) -> ArtifactDigest;
    /// Validate found and missing dependencies without treating unreadable files as missing.
    ///
    /// # Errors
    /// Refuses unsafe paths, changed bytes, conflicting roles and appeared missing paths.
    fn validate(&self, manifest: &ContextManifest, actual_root: &Path) -> Result<(), Error>;
    /// Seal actual existing corpus bytes without changing its filesystem or fabricating observations.
    ///
    /// # Errors
    /// Refuses missing, escaping, symlinked or duplicated inputs.
    fn seal_existing(
        &self,
        root: &Path,
        sources: &[ryo::linter::RelativePath],
        tracked: CorpusFiles<'_>,
    ) -> Result<ContextManifest, Error>;
    /// Materialize all bytes before either contestant runs and seal the exact lint input inventory.
    ///
    /// # Errors
    /// Refuses unsafe paths, an auxiliary file also requested for lint and failed writes.
    fn materialize(
        &self,
        boundary: &Path,
        context: &CaseContext,
        sources: &BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
    ) -> Result<ContextManifest, Error>;
    /// The receipt of the dependencies a run observed, from the manifest alone: it reads no file,
    /// so it costs the measured worker nothing per sealed byte. `verify_receipt` proves its bytes,
    /// absences and fingerprints against the snapshot.
    ///
    /// # Errors
    /// Refuses undeclared presences, absences contradicting a sealed file, escaping and duplicate
    /// dependencies.
    fn receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        dependencies: Vec<ryo::linter::ProjectDependency>,
    ) -> Result<ProjectContextReceipt, Error>;
    /// Verify the receipt after crossing the worker boundary, including its opaque fingerprints.
    /// Proves the snapshot (as `validate`), each walked dependency path, each present dependency's
    /// bytes against its fingerprint and each new absence, then requires equality with `receipt` of
    /// the same observations.
    ///
    /// # Errors
    /// Refuses a receipt that disagrees with the snapshot it claims.
    fn verify_receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        observed: &ProjectContextReceipt,
    ) -> Result<(), Error>;
    /// The canonical text of a receipt, as an observation document carries it.
    ///
    /// # Errors
    /// Refuses a receipt the document encoding cannot spell.
    fn receipt_text(&self, receipt: &ProjectContextReceipt) -> Result<CanonicalJson, Error>;
}
