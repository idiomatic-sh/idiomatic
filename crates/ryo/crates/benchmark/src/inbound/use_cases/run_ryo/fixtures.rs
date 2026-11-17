//! The worker's doubles at its context seam: a counting context and a sealed request.
use crate::outbound::context::{
    fixtures::fake::ContextFake, interface::ContextPort, types::CorpusFiles,
};
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::engine::types::loaded_engine_session::value::EngineExecution;
use crate::outbound::worker::fixtures::data::request;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::{
    CaseContext, ContextManifest, ProjectContextReceipt,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::sync::Arc;
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
                links: std::collections::BTreeMap::new(),
            },
        ),
        root: boundary,
        ..request()
    }
}

/// Where the engine double fails.
#[derive(Clone, Copy, Debug)]
pub enum Failing {
    /// Nowhere.
    Nothing,
    /// Loading a request.
    Load,
    /// Configuring it: no rule is loaded.
    Configure,
    /// Executing it.
    Execute,
    /// Compiling it ahead.
    Compile,
}

/// The report the engine double answers for a run over `files`.
pub fn report(files: Vec<ryo::linter::RelativePath>) -> RunReport {
    RunReport {
        execution_metadata:
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
        project_context:
            ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
        files,
        skipped: Vec::new(),
        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            },
        ),
        diagnostics: Vec::new(),
    }
}

/// The engine double: it counts loads, closes and runs, and fails where `failing` says.
#[derive(Debug)]
pub struct Engine {
    /// How many runs executed.
    pub calls: AtomicUsize,
    /// How many requests were loaded.
    pub loaded: AtomicUsize,
    /// How many loaded sessions were closed.
    pub closed: AtomicUsize,
    /// Where the engine fails.
    pub failing: Failing,
}
/// One loaded request of the engine double.
struct Session<'a> {
    engine: &'a Engine,
    request: &'a WorkerRequest,
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        let _previous = self.engine.closed.fetch_add(1, Ordering::SeqCst);
    }
}
impl EnginePort for Engine {
    fn compile(
        &self,
        _request: &WorkerRequest,
    ) -> Option<Result<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error>>
    {
        matches!(self.failing, Failing::Compile)
            .then(|| Err(Error::Invalid("fixture compile failure".into())))
    }
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<
        crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
        Error,
    > {
        let _previous = self.loaded.fetch_add(1, Ordering::SeqCst);
        if matches!(self.failing, Failing::Load) {
            return Err(Error::Invalid("fixture load failure".into()));
        }
        let rules = if matches!(self.failing, Failing::Configure) {
            Default::default()
        } else {
            crate::outbound::engine::fixtures::fake::loaded_rules(request)?
        };
        Ok(
            crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession::new(
                request,
                rules,
                None,
                Box::new(Session {
                    engine: self,
                    request,
                }),
            ),
        )
    }
}
impl EngineExecution for Session<'_> {
    fn configuration_files(&self) -> Result<Vec<ryo::linter::RelativePath>, Error> {
        Ok(vec![])
    }
    fn execute(
        self: Box<Self>,
        _rules: std::collections::BTreeMap<ryo::linter::RuleId, ryo::linter::Rule>,
        configured: crate::outbound::engine::types::EngineConfiguration,
    ) -> Result<crate::outbound::engine::types::EngineRun, Error> {
        assert_eq!(
            self.engine.closed.load(Ordering::SeqCst),
            0,
            "session stays live throughout SDK execution"
        );
        let _previous = self.engine.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.engine.failing, Failing::Execute) {
            return Err(Error::Invalid("fixture engine failure".into()));
        }
        assert_eq!(
            configured.selected_pairs,
            crate::outbound::engine::types::SelectedPairs(1)
        );
        assert!(configured.hosted.is_empty());
        Ok(crate::outbound::engine::types::EngineRun {
            dependencies: Vec::new(),
            report: report(self.request.files.clone()),
        })
    }
}
/// An engine that forwards to another through the public `EnginePort` contract alone: it cannot inspect or mutate the handle.
#[derive(Debug)]
pub struct ForwardingPort {
    /// The engine it forwards to.
    pub inner: Arc<Engine>,
    /// The request it loads instead, when it retargets.
    pub other: WorkerRequest,
    /// Whether it loads `other` in place of the request it is given.
    pub retarget: bool,
}
impl EnginePort for ForwardingPort {
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<
        crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
        Error,
    > {
        self.inner
            .load(if self.retarget { &self.other } else { request })
    }
}
