//! The context and linter doubles bracketing a candidate's launch.
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{ProjectContext, RunReport};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::collections::BTreeMap;
/// Which side of the bracket around a launch the context port refuses.
#[derive(Clone, Copy, Debug)]
pub(super) enum Refused {
    Seal,
    Receipt,
}
/// The context fake, refusing the corpus before the launch or the receipt after it.
#[derive(Debug)]
pub(super) struct RefusingContext(pub(super) Refused);
impl crate::outbound::context::interface::ContextPort for RefusingContext {
    fn fingerprint(
        &self,
        manifest: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
    ) -> ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest {
        crate::outbound::context::fixtures::fake::ContextFake.fingerprint(manifest)
    }
    fn validate(
        &self,
        _manifest: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
        _actual_root: &std::path::Path,
    ) -> Result<(), Error> {
        match self.0 {
            Refused::Seal => Err(Error::Invalid("fixture seal refusal".into())),
            Refused::Receipt => Ok(()),
        }
    }
    fn seal_existing(
        &self,
        root: &std::path::Path,
        sources: &[ryo::linter::RelativePath],
        tracked: crate::outbound::context::types::CorpusFiles<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ContextManifest, Error> {
        crate::outbound::context::fixtures::fake::ContextFake.seal_existing(root, sources, tracked)
    }
    fn materialize(
        &self,
        boundary: &std::path::Path,
        context: &ryo_benchmark_contracts::domain::context::types::CaseContext,
        sources: &BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ContextManifest, Error> {
        crate::outbound::context::fixtures::fake::ContextFake
            .materialize(boundary, context, sources)
    }
    fn receipt(
        &self,
        manifest: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
        actual_root: &std::path::Path,
        dependencies: Vec<ryo::linter::ProjectDependency>,
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt, Error> {
        crate::outbound::context::fixtures::fake::ContextFake.receipt(
            manifest,
            actual_root,
            dependencies,
        )
    }
    fn verify_receipt(
        &self,
        _manifest: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
        _actual_root: &std::path::Path,
        _observed: &ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt,
    ) -> Result<(), Error> {
        Err(Error::Invalid("fixture receipt refusal".into()))
    }
    fn receipt_text(
        &self,
        receipt: &ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt,
    ) -> Result<ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson, Error>
    {
        crate::outbound::context::fixtures::fake::ContextFake.receipt_text(receipt)
    }
}
/// The linter fake, decoding every report as one that observed its sealed context.
#[derive(Debug, Default)]
pub(super) struct ObservingLinter(crate::outbound::linter::fixtures::fake::LinterFake);
impl LinterPort for ObservingLinter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        self.0.rule_configuration()
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        self.0.prepare(request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = self.0.decode(request, output)?;
        report.project_context = ProjectContext::Observed(
            ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt {
                mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk,
                logical_root: ".".into(),
                actual_root: request.root.clone(),
                manifest_hash:
                    ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                        "fixture".into(),
                    ),
                dependencies: BTreeMap::new(),
            },
        );
        Ok(report)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<crate::outbound::linter::types::CanonicalText, Error> {
        self.0.settings_text(normalized, case, profile)
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        self.0.rule_context_text(prepared)
    }
    fn version_line(
        &self,
        tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine {
        self.0.version_line(tool)
    }
}
/// A sealed context manifest; the refusing context never reads it.
pub(super) fn sealed_manifest() -> ryo_benchmark_contracts::domain::context::types::ContextManifest
{
    ryo_benchmark_contracts::domain::context::types::ContextManifest {
        boundary: "/fixture".into(),
        logical_root: ".".into(),
        auxiliary_files: BTreeMap::new(),
        source_files: BTreeMap::new(),
        missing: Vec::new(),
    }
}
