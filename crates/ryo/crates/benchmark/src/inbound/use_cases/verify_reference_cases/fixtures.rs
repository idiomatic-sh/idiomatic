//! Independent source inputs through the actual public use case and real process boundary.
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// The public operation under test, given the ports, the rules and the scratch directory.
pub type Operate =
    fn(&dyn SuitePort, &dyn LinterPort, &dyn ExecutionPort, &Path, &Path) -> Result<usize, ReportsError>;
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Complete,
    Missing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MissingSource,
    Unexpected,
}
/// Run one public operation over independently authored source facts.
pub fn exercise(scenario: Scenario, operate: Operate) -> Result<usize, Refusal> {
    crate::inbound::use_cases::prepare_cases::fixtures::with_verification_sources(
        |suite, reference, _candidate, execution, rules, scratch, launches| {
            let input = match scenario {
                Scenario::Complete => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, reference, execution, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap().lines().count(), 2);
                }
                Scenario::Missing => assert_eq!(
                    std::fs::symlink_metadata(launches).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                ),
            }
            result.map_err(|error| match error {
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Refusal::MissingSource
                }
                _ => Refusal::Unexpected,
            })
        },
    )
}

/// Independent authored evidence for the reference-only verifier, constructed apart from the reference's response.
#[cfg(test)]
pub(crate) mod authored {
    use super::super::service::verify_prepared_reference_cases;
    use crate::outbound::{
        execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
        receipts::fixtures::fake::ReceiptsFake, suite::fixtures::fake::SuiteFake,
    };
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
    use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
    use std::path::Path;

    /// The shape of the authored suite the reference answers.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Shape {
        Agreement,
        NoCases,
        RuleProjection,
    }

    /// Names what an error means by its message, so rows state the refusal and not its wording.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum Refusal {
        ReferenceMismatch,
        ReferenceStartup,
        UnprovenEvidence,
        NoCases,
        Unexpected,
    }

    /// The refusal an error message states.
    pub(crate) fn category(message: &str) -> Refusal {
        if message.contains("expected-vs-reference") {
            Refusal::ReferenceMismatch
        } else if message.contains("reference: operational failure") {
            Refusal::ReferenceStartup
        } else if message == crate::domain::suite::constants::NO_CASES {
            Refusal::NoCases
        } else if message.contains("fixture has no independent discrepancy artifacts") {
            Refusal::UnprovenEvidence
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

    /// Construct authored evidence independently from the reference's response.
    pub(crate) fn prepared(shape: Shape) -> (SuiteFake, BenchmarkFinding) {
        let expected = authored::finding("E722", 0, 6);
        let mut suite = SuiteFake::default();
        if !matches!(shape, Shape::NoCases) {
            suite.prepared.push(authored::case(&expected));
        }
        suite.profile = Some(authored::profile());
        (suite, expected)
    }

    /// Give the reference parser diagnostics and an unrelated lint as a control, and the authored case the parser diagnostic.
    pub(crate) fn project_rules(suite: &mut SuiteFake, reference: &mut LinterFake) {
        let frontend = authored::finding("invalid-syntax", 9, 19);
        reference.diagnostics.push(frontend.clone());
        reference.diagnostics.push(authored::finding("F401", 20, 24));
        suite
            .prepared
            .first_mut()
            .expect("authored case")
            .expected
            .diagnostics
            .push(frontend);
    }

    /// The reference-only verification of every prepared case of `suite`, with a reference answering `reference`.
    pub(crate) fn verify_reference(
        suite: &SuiteFake,
        reference: &LinterFake,
        execution: &ExecutionFake,
        rules: &Path,
    ) -> Result<usize, ReportsError> {
        verify_prepared_reference_cases(
            suite,
            &ReceiptsFake::default(),
            &crate::outbound::context::factory::local(),
            reference,
            execution,
            &crate::outbound::inventory::factory::local(),
            rules,
            suite.prepared.clone(),
        )
        .map(|count| count.0)
    }
}

/// A documented reference difference keeps the intended contract and the pinned reference apart.
#[cfg(test)]
pub(crate) mod discrepancy {
    use super::authored::{Shape, prepared};
    use crate::outbound::{
        linter::{fixtures::fake::LinterFake, interface::LinterPort},
        suite::fixtures::fake::SuiteFake,
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::requests::types::{
        ParserSelection, ProviderSelection, SearchRoots,
    };
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, DiscrepancyBasis, DiscrepancyReason, FileObservation,
        MetadataDigestDeclaration, PreparedReferenceBehavior, PreparedReferenceDiscrepancy,
        ReferenceDiscrepancyIdentity, RuleContextProfile, Tool,
        discrepancy_evidence::value::{
            DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
        },
    };

    /// A suite whose one case documents how the pinned reference differs from the intended behavior.
    pub(crate) fn discrepancy_suite() -> SuiteFake {
        let (mut suite, _) = prepared(Shape::Agreement);
        let case = suite.prepared.first_mut().expect("authored case");
        case.comparison = CaseComparison::Full;
        case.request.root = "/case".into();
        let mut reference = case.expected.clone();
        reference.diagnostics.clear();
        let profile_hash = ArtifactDigest("a".repeat(64));
        let binary_hash = ArtifactDigest("b".repeat(64));
        let profile = Tool {
            rule_observation_selection: ObservationSelectionPolicy::Declared,
            supporting_rules: Vec::new(),
            rule_observations: Default::default(),
            rule_context: RuleContextProfile::Undeclared,
            file_observation: FileObservation::Undeclared,
            project_context_mode: ProjectContextMode::Isolated,
            tool: "fixture".into(),
            version: "1".into(),
            sha256: vec![binary_hash.0.clone().into()],
            defaults: vec![case.rule.clone()],
            deferred: std::collections::BTreeMap::default(),
            pending: std::collections::BTreeMap::default(),
            query_modules: std::collections::BTreeMap::default(),
            rule_applicability: std::collections::BTreeMap::default(),
            parser_configuration: ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(
                "python-target=3.10".into(),
            )),
            source_language: ProviderSelection::Unspecified,
            project_search_roots: SearchRoots::Undeclared,
        };
        let invocation = LinterFake::default()
            .prepare(&case.request)
            .expect("fixture invocation");
        let settings = crate::domain::canonical_text::service::text_digest(
            &crate::application::discrepancy_validation::service::settings_text(
                &crate::outbound::linter::fixtures::fake::LinterFake::default(),
                &invocation,
                case,
                &profile,
            )
            .expect("fixture settings"),
        );
        case.reference_discrepancy = PreparedReferenceBehavior::Differs(PreparedReferenceDiscrepancy {
            basis: DiscrepancyBasis::LanguageDefect,
            identity: ReferenceDiscrepancyIdentity {
                file_metadata_sha256: MetadataDigestDeclaration::Undeclared,
                tool: "fixture".into(),
                version: "1".into(),
                observed_executable_sha256: binary_hash.clone(),
                profile_sha256: profile_hash.clone(),
                settings_sha256: settings,
                source_sha256: ArtifactDigest("c".repeat(64)),
                intended_output_sha256: crate::domain::discrepancy::service::output_digest(
                    &case.expected,
                ),
                reference_output_sha256: crate::domain::discrepancy::service::output_digest(&reference),
            },
            expected:
                ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::Diagnostics(
                    reference,
                ),
            reason: DiscrepancyReason(
                "independent language contract differs from the pinned reference".into(),
            ),
            evidence: vec![DiscrepancyEvidence::Language {
                path: "evidence.json".into(),
                sha256: ArtifactDigest("d".repeat(64)),
                kind: DiscrepancyEvidenceKind::IndependentLanguageRun,
                language_version: LanguageVersion("3.10".into()),
                target_configuration: ParserTarget("python-target=3.10".into()),
                version_rationale: Rationale("exact target independently evaluated".into()),
            }],
        });
        suite.profile = Some(profile);
        let _previous_profile = suite.artifact_digests.insert(
            std::path::Path::new("/profile")
                .join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
            profile_hash,
        );
        let _previous_binary = suite.artifact_digests.insert("fixture".into(), binary_hash);
        suite.discrepancy_artifacts_valid = true;
        let authored_case = suite.prepared.first_mut().expect("authored case");
        let PreparedReferenceBehavior::Differs(item) = std::mem::replace(
            &mut authored_case.reference_discrepancy,
            PreparedReferenceBehavior::Agrees,
        ) else {
            unreachable!("fixture canonical certificate")
        };
        let identity = item.identity;
        let discrepancy = {
            use crate::outbound::suite::types::{
                ClaimedDiscrepancy, ClaimedEvidence, ClaimedIdentity, ClaimedMetadata, ClaimedProcess,
            };
            use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
            use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
                LanguageVersion, ParserTarget, Rationale,
            };
            ClaimedDiscrepancy {
                process: ClaimedProcess::Absent,
                expected: Vec::new(),
                identity: ClaimedIdentity {
                    file_metadata_sha256: ClaimedMetadata::Undeclared,
                    tool: identity.tool,
                    version: identity.version,
                    observed_executable_sha256: Sha256Text(identity.observed_executable_sha256.0),
                    profile_sha256: Sha256Text(identity.profile_sha256.0),
                    settings_sha256: Sha256Text(identity.settings_sha256.0),
                    source_sha256: Sha256Text(identity.source_sha256.0),
                    reference_output_sha256: Sha256Text(identity.reference_output_sha256.0),
                    intended_output_sha256: Sha256Text(identity.intended_output_sha256.0),
                },
                evidence: vec![ClaimedEvidence::IndependentLanguageRun {
                    path: "evidence.json".into(),
                    sha256: Sha256Text("d".repeat(64)),
                    language_version: LanguageVersion("3.10".into()),
                    target_configuration: ParserTarget("python-target=3.10".into()),
                    version_rationale: Rationale("exact target independently evaluated".into()),
                }],
                reason: item.reason,
            }
        };
        suite.reference_claims.push(
            crate::outbound::suite::types::CaseClaim {
                rule: ryo_benchmark_contracts::domain::documents::types::text::value::RuleName(authored_case.rule.to_string()),
                name: ryo_benchmark_contracts::domain::documents::types::text::value::CaseName(authored_case.name.to_string()),
                basis: ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
                discrepancy,
            },
        );
        suite
    }
}
