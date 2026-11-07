//! A prepared case with a distinct pinned reference, and the profile that pins its tool.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
    RuleCoverage, RunReport, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    ParserConfigurationName, ParserSelection, ProviderSelection, RuleContextPreparation,
    RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, DiscrepancyBasis, DiscrepancyReason, FileObservation,
    MetadataDigestDeclaration, PreparedCase, PreparedReferenceBehavior,
    PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity, RuleContextProfile, Tool,
    discrepancy_evidence::value::{
        DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
    },
};
use std::collections::BTreeMap;

/// A complete report with one file and no findings.
pub(crate) fn report() -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        diagnostics: Vec::new(),
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        files: vec![ryo::linter::RelativePath("case.py".into())],
    }
}

/// The identity both pins and certificates of the fixture share.
pub(crate) fn identity() -> ReferenceDiscrepancyIdentity {
    ReferenceDiscrepancyIdentity {
        file_metadata_sha256: MetadataDigestDeclaration::Undeclared,
        tool: "fixture".into(),
        version: "1.0".into(),
        observed_executable_sha256: ArtifactDigest("a".repeat(64)),
        profile_sha256: ArtifactDigest("c".repeat(64)),
        settings_sha256: ArtifactDigest("d".repeat(64)),
        source_sha256: ArtifactDigest("e".repeat(64)),
        reference_output_sha256: ArtifactDigest("f".repeat(64)),
        intended_output_sha256: ArtifactDigest("0".repeat(64)),
    }
}

/// Minimal distinct intended and reference outputs with complete input coverage.
pub(crate) fn case() -> PreparedCase {
    let expected = report();
    let mut reference = expected.clone();
    reference.diagnostics.push(BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
        file: expected.files[0].clone(),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    });
    PreparedCase {
        comparison: CaseComparison::Full,
        rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
        name: "independent-contract".into(),
        request: RunRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Frozen,
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            staged_sources: StagedSources::Unlisted,
            file_metadata: FileMetadataDeclaration::Undeclared,
            root: ".".into(),
            state: "cache".into(),
            files: expected.files.clone(),
            context_manifest: ContextSeal::Unsealed,
        },
        expected,
        reference_discrepancy: PreparedReferenceBehavior::Differs(PreparedReferenceDiscrepancy {
            basis: DiscrepancyBasis::LanguageDefect,
            expected: ReferenceExpectation::Diagnostics(reference),
            identity: identity(),
            evidence: Vec::new(),
            reason: DiscrepancyReason("independent language contract".into()),
        }),
    }
}

/// The profile pinning the fixture tool to two approved native binaries.
pub(crate) fn profile() -> Tool {
    Tool {
        rule_observation_selection: ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: RuleContextProfile::Undeclared,
        file_observation: FileObservation::Undeclared,
        project_context_mode: ProjectContextMode::Isolated,
        tool: "fixture".into(),
        version: "1.0".into(),
        sha256: vec!["a".repeat(64).into(), "b".repeat(64).into()],
        defaults: Vec::new(),
        deferred: BTreeMap::default(),
        pending: BTreeMap::default(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName("python3.10".into())),
        source_language: ProviderSelection::Unspecified,
        project_search_roots: SearchRoots::Undeclared,
    }
}

/// Independent language meaning for the fixture profile's parser target.
pub(crate) fn evidence() -> DiscrepancyEvidence {
    DiscrepancyEvidence::Language {
        path: "semantic-authority.txt".into(),
        sha256: ArtifactDigest("f".repeat(64)),
        kind: DiscrepancyEvidenceKind::LanguageSpec,
        language_version: LanguageVersion("toy-v1".into()),
        target_configuration: ParserTarget("python3.10".into()),
        version_rationale: Rationale("toy fixture contract".into()),
    }
}

/// The documented reference difference of a case that has one.
///
/// # Panics
/// When the case's reference agrees with the intended behavior.
pub(crate) fn differing(case: &mut PreparedCase) -> &mut PreparedReferenceDiscrepancy {
    match &mut case.reference_discrepancy {
        PreparedReferenceBehavior::Differs(item) => item,
        PreparedReferenceBehavior::Agrees => panic!("the fixture case documents a difference"),
    }
}

/// The accounted coverage of a report.
///
/// # Panics
/// When the report carries no accounting.
pub(crate) fn accounted(report: &mut RunReport) -> &mut RuleCoverage {
    match &mut report.coverage {
        CoverageAccounting::Accounted(coverage) => coverage,
        CoverageAccounting::Unaccounted => panic!("the fixture report accounts its coverage"),
    }
}
