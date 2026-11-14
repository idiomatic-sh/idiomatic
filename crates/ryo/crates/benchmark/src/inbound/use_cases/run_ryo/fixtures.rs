//! The worker's doubles at its context seam: a counting context and a sealed request.
use crate::outbound::context::{
    fixtures::fake::ContextFake, interface::ContextPort, types::CorpusFiles,
};
use crate::outbound::worker::fixtures::data::request;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::{
    CaseContext, ContextManifest, ProjectContextReceipt,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::sync::atomic::{AtomicUsize, Ordering};
/// The context fake, counting how often the worker validates and receipts through the port.
#[derive(Debug, Default)]
pub struct CountingContext {
    /// The fake every call is forwarded to.
    inner: ContextFake,
    /// How often the corpus was validated.
    pub validated: AtomicUsize,
    /// How often a receipt was assembled.
    pub receipted: AtomicUsize,
}
impl ContextPort for CountingContext {
    fn fingerprint(&self, manifest: &ContextManifest) -> ArtifactDigest {
        self.inner.fingerprint(manifest)
    }
    fn validate(
        &self,
        manifest: &ContextManifest,
        actual_root: &std::path::Path,
    ) -> Result<(), Error> {
        let _prior = self.validated.fetch_add(1, Ordering::SeqCst);
        self.inner.validate(manifest, actual_root)
    }
    fn seal_existing(
        &self,
        root: &std::path::Path,
        sources: &[ryo::linter::RelativePath],
        tracked: CorpusFiles<'_>,
    ) -> Result<ContextManifest, Error> {
        self.inner.seal_existing(root, sources, tracked)
    }
    fn materialize(
        &self,
        boundary: &std::path::Path,
        context: &CaseContext,
        sources: &std::collections::BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
    ) -> Result<ContextManifest, Error> {
        self.inner.materialize(boundary, context, sources)
    }
    fn receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &std::path::Path,
        dependencies: Vec<ryo::linter::ProjectDependency>,
    ) -> Result<ProjectContextReceipt, Error> {
        let _prior = self.receipted.fetch_add(1, Ordering::SeqCst);
        self.inner.receipt(manifest, actual_root, dependencies)
    }
    fn verify_receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &std::path::Path,
        observed: &ProjectContextReceipt,
    ) -> Result<(), Error> {
        self.inner.verify_receipt(manifest, actual_root, observed)
    }
    fn receipt_text(&self, receipt: &ProjectContextReceipt) -> Result<CanonicalJson, Error> {
        self.inner.receipt_text(receipt)
    }
}
/// The request fixture, sealed over its one source file in listed-disk mode.
///
/// # Panics
/// Never: the fixture boundary is a relative literal the working directory makes absolute.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the fixture boundary is a relative literal, so making it absolute cannot fail"
)]
pub fn sealed_request() -> WorkerRequest {
    let boundary = std::path::absolute("workspace").expect("an absolute fixture boundary");
    WorkerRequest {
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(
            ContextManifest {
                boundary: boundary.clone(),
                logical_root: ".".into(),
                auxiliary_files: std::collections::BTreeMap::new(),
                source_files: std::collections::BTreeMap::from([(
                    "source.txt".into(),
                    ArtifactDigest("fixture".into()),
                )]),
                missing: Vec::new(),
            },
        ),
        root: boundary,
        ..request()
    }
}
