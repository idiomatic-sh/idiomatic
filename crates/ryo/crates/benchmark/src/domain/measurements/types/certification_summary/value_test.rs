//! The certification line states verified, deferred, pending and unassessed rules, the cases and the certificate.
use super::super::rule_tally::value::RuleTally;
use super::value::CertificationSummary;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::{audit::value::Audit, certificate::value::Certificate};
use ryo_benchmark_contracts::domain::requests::types::{
    BenchmarkCoverage, CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount;

/// The scope the summary states.
type Input = BenchmarkCoverage;
/// The line, as written.
type Value = &'static str;

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName(
            "an interim certification of one of two authored rules out of five defaults",
        ),
        word: Word::Succeeds,
        input: || BenchmarkCoverage::Interim,
        expect: Expect::Value(
            "scope: Interim; 1 verified / 5 shipped defaults; 1 assessed deferrals; 1 assessed pending; 1 unassessed; 1 authored outside subset; 2 cases; certificate=sealed; artifacts=profile",
        ),
    },
    Case {
        name: CaseName("an accounted certification names its scope Accounted"),
        word: Word::Succeeds,
        input: || BenchmarkCoverage::Accounted,
        expect: Expect::Value(
            "scope: Accounted; 1 verified / 5 shipped defaults; 1 assessed deferrals; 1 assessed pending; 1 unassessed; 1 authored outside subset; 2 cases; certificate=sealed; artifacts=profile",
        ),
    },
];

/// A certificate holding two cases and a fixed fingerprint.
fn certificate(coverage: Audit) -> Certificate {
    let probe = RunRequest {
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
        files: vec![],
        state: "cache".into(),
    };
    let profile = || ryo_benchmark_contracts::domain::measurements::types::Invocation {
        executable: "fixture".into(),
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::new(),
        directory: ".".into(),
    };
    Certificate {
        case_source: ryo_benchmark_contracts::domain::requests::types::CertificationCases::Profile,
        coverage,
        paths: vec![],
        fingerprint:
            ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                "sealed".into(),
            ),
        contestants: ["first".into(), "second".into()],
        probe,
        profiles: [profile(), profile()],
        execution: ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Wall,
        cases: ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scope| {
        let rule = |id| ryo::linter::RuleId::try_from(id).map_err(|_invalid| ());
        let audit = Audit {
            implemented: vec![rule("A")?, rule("B")?],
            missing: vec![rule("C")?],
        };
        let verified = audit.select(&[rule("A")?]).map_err(|_refused| ())?;
        let tally = RuleTally {
            defaults: RuleCount(5),
            deferred: RuleCount(1),
            pending: RuleCount(1),
            authored: RuleCount(2),
        };
        let certificate = certificate(audit);
        let summary = CertificationSummary {
            scope,
            tally: &tally,
            audit: &verified,
            certificate: &certificate,
            profile: std::path::Path::new("profile"),
        };
        Ok(&*Box::leak(summary.to_string().into_boxed_str()))
    });
}
