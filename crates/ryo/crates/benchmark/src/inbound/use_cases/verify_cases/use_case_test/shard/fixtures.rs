//! Independent authored evidence, constructed apart from either contestant response.
use crate::outbound::{execution::fixtures::fake::ExecutionFake, suite::fixtures::fake::SuiteFake};
use crate::outbound::linter::fixtures::fake::LinterFake;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
    RuleCoverage, RunReport, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, ParserSelection,
    ProviderSelection, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, FileObservation, PreparedCase,
    PreparedReferenceBehavior, RuleContextProfile, Tool,
};

/// The shape of the authored suite and of the findings both contestants report.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    Agreement,
    MissingCandidate,
    MissingReference,
    NoCases,
    StartupFailure,
    FullAgreement,
    MissingFrontend,
    RuleProjection,
}

/// Names what an error means by its message, so rows state the refusal and not its wording.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    CandidateMismatch,
    ReferenceMismatch,
    CandidateStartup,
    ReferenceStartup,
    UnaccountedDefault,
    NoCases,
    InputCoverage,
    UnprovenEvidence,
    MetadataDrift,
    VerificationRequired,
    Unexpected,
}

/// The refusal an error message states.
pub(crate) fn category(message: &str) -> Refusal {
    if message.contains("expected-vs-candidate") {
        Refusal::CandidateMismatch
    } else if message.contains("expected-vs-reference") {
        Refusal::ReferenceMismatch
    } else if message == "operational failure" {
        Refusal::CandidateStartup
    } else if message.contains("reference: operational failure") {
        Refusal::ReferenceStartup
    } else if message.contains("defaults lack implementation or explicit deferral") {
        Refusal::UnaccountedDefault
    } else if message == crate::domain::suite::constants::NO_CASES {
        Refusal::NoCases
    } else if message == crate::domain::diagnostics::constants::INPUT_COVERAGE {
        Refusal::InputCoverage
    } else if message.contains("fixture has no independent discrepancy artifacts") {
        Refusal::UnprovenEvidence
    } else if message.contains("executable metadata drifted") {
        Refusal::MetadataDrift
    } else if message.contains("require verification preparation") {
        Refusal::VerificationRequired
    } else {
        Refusal::Unexpected
    }
}

/// The number of launches an execution fake recorded.
pub(crate) fn history_len(execution: &ExecutionFake) -> Option<usize> {
    execution.history.lock().ok().map(|history| history.len())
}

/// An execution that exits cleanly.
pub(crate) fn succeeding() -> ExecutionFake {
    ExecutionFake {
        status: 0,
        ..Default::default()
    }
}

/// The complete one-file report with `diagnostics`.
fn report(diagnostics: Vec<BenchmarkFinding>) -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        files: vec![ryo::linter::RelativePath("case.py".into())],
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        diagnostics,
    }
}

/// A finding of `rule` over `start..end` bytes of the case file.
pub(crate) fn finding(rule: &str, start: usize, end: usize) -> BenchmarkFinding {
    BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(rule).expect("fixture rule identity"),
        file: ryo::linter::RelativePath("case.py".into()),
        start: SourceByteOffset(start),
        end: SourceByteOffset(end),
    }
}

/// A request over the one case file.
pub(crate) fn request() -> RunRequest {
    RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: ".".into(),
        files: vec![ryo::linter::RelativePath("case.py".into())],
        state: "cache".into(),
    }
}

/// Construct authored evidence independently from either contestant response.
pub(crate) fn prepared(shape: Shape) -> (SuiteFake, BenchmarkFinding) {
    let expected = finding("E722", 0, 6);
    let mut suite = SuiteFake::default();
    if !matches!(shape, Shape::NoCases) {
        suite.prepared.push(PreparedCase {
            reference_discrepancy: PreparedReferenceBehavior::Agrees,
            comparison: CaseComparison::Rule,
            rule: ryo::linter::RuleId::try_from("E722").unwrap(),
            name: "positive".into(),
            request: request(),
            expected: report(vec![expected.clone()]),
        });
    }
    suite.profile = Some(Tool {
        rule_observation_selection: ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: RuleContextProfile::Undeclared,
        file_observation: FileObservation::Undeclared,
        project_context_mode: ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: ParserSelection::Unspecified,
        source_language: ProviderSelection::Unspecified,
        project_search_roots: SearchRoots::Undeclared,
    });
    (suite, expected)
}

/// Give both contestants parser diagnostics, optionally retaining an unrelated lint as a control.
pub(crate) fn report_scope(
    shape: Shape,
    suite: &mut SuiteFake,
    reference: &mut LinterFake,
    candidate: &mut LinterFake,
) {
    if matches!(
        shape,
        Shape::FullAgreement | Shape::MissingFrontend | Shape::RuleProjection
    ) {
        let frontend = finding("invalid-syntax", 9, 19);
        reference.diagnostics.push(frontend.clone());
        if !matches!(shape, Shape::MissingFrontend) {
            candidate.diagnostics.push(frontend.clone());
        }
        if matches!(shape, Shape::RuleProjection) {
            let unrelated = finding("F401", 20, 24);
            reference.diagnostics.push(unrelated.clone());
            candidate.diagnostics.push(unrelated);
        }
        let authored = suite.prepared.first_mut().expect("authored case");
        if !matches!(shape, Shape::RuleProjection) {
            authored.comparison = CaseComparison::Full;
        }
        authored.expected.diagnostics.push(frontend);
    }
}
