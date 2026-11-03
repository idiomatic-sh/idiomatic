//! Independent output contracts and native executable portability remain strict.
use super::*;
use crate::domain::suite::types::{
    PreparedReferenceDiscrepancy, ProjectContextMode, ReferenceDiscrepancyIdentity, RunRequest,
    Tool,
};

/// Minimal distinct intended/reference outputs with complete input coverage.
fn case() -> PreparedCase {
    let report = RunReport {
        execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
        project_context: None,
        diagnostics: Vec::new(),
        skipped: Vec::new(),
        coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        files: vec![ryo::linter::RelativePath("case.py".into())],
    };
    let mut reference = report.clone();
    reference
        .diagnostics
        .push(crate::domain::diagnostics::types::BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
            file: report.files[0].clone(),
            start: crate::domain::diagnostics::types::SourceByteOffset(0),
            end: crate::domain::diagnostics::types::SourceByteOffset(1),
        });
    PreparedCase {
        comparison: CaseComparison::Full,
        rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
        name: "independent-contract".into(),
        request: RunRequest {
            file_metadata: None,
            root: ".".into(),
            state: "cache".into(),
            files: report.files.clone(),
            context_manifest: None,
        },
        expected: report,
        reference_discrepancy: Some(PreparedReferenceDiscrepancy {
            basis: crate::domain::suite::types::DiscrepancyBasis::LanguageDefect,
            expected: crate::domain::suite::process::types::ReferenceExpectation::Diagnostics(
                reference,
            ),
            identity: identity(),
            evidence: Vec::new(),
            reason: "independent language contract".into(),
        }),
    }
}

/// Distinct native approved digests model different operating system distributions.
fn identity() -> ReferenceDiscrepancyIdentity {
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

#[test]
fn wrong_oracle_cannot_satisfy_the_intended_candidate_contract() {
    let case = case();
    let reference = reference_expected(&case).expect("diagnostic expectation");
    assert!(
        compare(reference, reference).is_ok(),
        "pinned reference remains exact"
    );
    assert!(
        compare(&case.expected, reference).is_err(),
        "copying wrong oracle must fail intended behavior"
    );
    assert_ne!(
        output_digest(&case.expected),
        output_digest(reference),
        "distinct contracts have distinct hashes"
    );
    let mut changed = case.expected.clone();
    changed
        .files
        .push(ryo::linter::RelativePath("other.py".into()));
    assert_ne!(
        output_digest(&changed),
        output_digest(&case.expected),
        "coverage identity changes invalidate evidence"
    );
}

#[test]
fn approved_platform_digest_can_differ_from_observed_evidence_binary() {
    let identity = identity();
    let profile = Tool {
        file_observation: None,
        project_context_mode: ProjectContextMode::Isolated,
        tool: "fixture".into(),
        version: "1.0".into(),
        sha256: vec!["a".repeat(64).into(), "b".repeat(64).into()],
        defaults: Vec::new(),
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: Some("python3.10".into()),
        source_language: None,
    };
    assert!(
        validate_pins(
            &identity,
            &profile,
            &ArtifactDigest("b".repeat(64)),
            &identity.profile_sha256
        )
        .is_ok(),
        "approved other platform is portable"
    );
    assert!(
        validate_pins(
            &identity,
            &profile,
            &ArtifactDigest("9".repeat(64)),
            &identity.profile_sha256
        )
        .is_err(),
        "unapproved tool cannot attest evidence"
    );
    assert!(
        validate_pins(
            &identity,
            &profile,
            &ArtifactDigest("a".repeat(64)),
            &ArtifactDigest("8".repeat(64))
        )
        .is_err(),
        "changed profile invalidates evidence"
    );
}

#[test]
fn unknown_missing_and_inconsistent_accounting_never_admit_discrepancies() {
    let case = case();
    let mut report = case.expected.clone();
    report.coverage = None;
    assert!(
        complete(&case, &report, true).is_err(),
        "candidate accounting is mandatory"
    );
    report.coverage = Some(crate::domain::diagnostics::types::RuleCoverage {
        selected: 1,
        evaluated: 1,
        skipped: 0,
        unknown_or_unread: 0,
    });
    assert!(
        complete(&case, &report, true).is_ok(),
        "complete candidate is admissible"
    );
    for unavailable in [
        crate::domain::diagnostics::types::ExecutionMetadata::Unknown,
        crate::domain::diagnostics::types::ExecutionMetadata::Unavailable {
            reason: "not observed".into(),
        },
    ] {
        report.execution_metadata = unavailable;
        assert!(
            complete(&case, &report, true).is_err(),
            "candidate capability must be explicitly complete"
        );
    }
    report.execution_metadata = crate::domain::diagnostics::types::ExecutionMetadata::Complete;
    report
        .coverage
        .as_mut()
        .expect("coverage")
        .unknown_or_unread = 1;
    assert!(
        complete(&case, &report, true).is_err(),
        "unknown is not a discrepancy"
    );
}

#[test]
fn missing_evidence_rule_projection_and_vacuous_outputs_fail_before_execution() {
    let suite = crate::outbound::suite::fixtures::fake::SuiteFake::default();
    let reference = crate::outbound::linter::fixtures::fake::LinterFake::default();
    let mut case = case();
    assert!(
        validate(&suite, &reference, Path::new("rules"), &case).is_err(),
        "missing evidence fails"
    );
    case.comparison = CaseComparison::Rule;
    assert!(
        validate(&suite, &reference, Path::new("rules"), &case).is_err(),
        "projection cannot hide differences"
    );
    case.comparison = CaseComparison::Full;
    case.reference_discrepancy
        .as_mut()
        .expect("discrepancy")
        .expected = crate::domain::suite::process::types::ReferenceExpectation::Diagnostics(
        case.expected.clone(),
    );
    assert!(
        validate(&suite, &reference, Path::new("rules"), &case).is_err(),
        "identical outputs are not a discrepancy"
    );
}

#[test]
fn canonical_unicode_multisets_match_the_python_certificate_encoding() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/reference-discrepancy-canonical.json"
    ))
    .expect("cross-language fixture");
    let value = &fixture["output"];
    assert_eq!(
        serde_json::to_string(value).expect("canonical JSON"),
        fixture["canonical_utf8"].as_str().expect("UTF8 bytes")
    );
    assert_eq!(
        json_digest(value).0,
        fixture["sha256"].as_str().expect("pinned Python hash")
    );
    let mut report = case().expected;
    report.files = vec![
        ryo::linter::RelativePath("雪/é.py".into()),
        ryo::linter::RelativePath("z.py".into()),
    ];
    let finding = crate::domain::diagnostics::types::BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
        file: report.files[0].clone(),
        start: crate::domain::diagnostics::types::SourceByteOffset(2),
        end: crate::domain::diagnostics::types::SourceByteOffset(7),
    };
    report.diagnostics = vec![finding.clone(), finding];
    assert_eq!(
        output_digest(&report).0,
        fixture["sha256"]
            .as_str()
            .expect("same normalized finding order and multiplicity")
    );
}
