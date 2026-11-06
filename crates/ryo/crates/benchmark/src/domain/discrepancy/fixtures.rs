//! A prepared case with a distinct pinned reference, and the profile that pins its tool.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, ExecutionMetadata, RuleCoverage, RunReport, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, DiscrepancyBasis, ObservationSelectionPolicy, PreparedCase,
    PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity, Tool,
    discrepancy_evidence::value::{DiscrepancyEvidence, DiscrepancyEvidenceKind},
};
use std::collections::BTreeMap;

/// A complete report with one file and no findings.
pub(crate) fn report() -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: None,
        diagnostics: Vec::new(),
        skipped: Vec::new(),
        coverage: Some(RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        files: vec![ryo::linter::RelativePath("case.py".into())],
    }
}

/// The identity both pins and certificates of the fixture share.
pub(crate) fn identity() -> ReferenceDiscrepancyIdentity {
    ReferenceDiscrepancyIdentity {
        file_metadata_sha256: None,
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
            selected_rules: None,
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            staged_sources: None,
            file_metadata: None,
            root: ".".into(),
            state: "cache".into(),
            files: expected.files.clone(),
            context_manifest: None,
        },
        expected,
        reference_discrepancy: Some(PreparedReferenceDiscrepancy {
            basis: DiscrepancyBasis::LanguageDefect,
            expected: ReferenceExpectation::Diagnostics(reference),
            identity: identity(),
            evidence: Vec::new(),
            reason: "independent language contract".into(),
        }),
    }
}

/// The profile pinning the fixture tool to two approved native binaries.
pub(crate) fn profile() -> Tool {
    Tool {
        rule_observation_selection: ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode: ProjectContextMode::Isolated,
        tool: "fixture".into(),
        version: "1.0".into(),
        sha256: vec!["a".repeat(64).into(), "b".repeat(64).into()],
        defaults: Vec::new(),
        deferred: BTreeMap::default(),
        pending: BTreeMap::default(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: Some("python3.10".into()),
        source_language: None,
        project_search_roots: None,
    }
}

/// Independent language meaning for the fixture profile's parser target.
pub(crate) fn evidence() -> DiscrepancyEvidence {
    DiscrepancyEvidence::Language {
        path: "semantic-authority.txt".into(),
        sha256: ArtifactDigest("f".repeat(64)),
        kind: DiscrepancyEvidenceKind::LanguageSpec,
        language_version: "toy-v1".into(),
        target_configuration: "python3.10".into(),
        version_rationale: "toy fixture contract".into(),
    }
}
