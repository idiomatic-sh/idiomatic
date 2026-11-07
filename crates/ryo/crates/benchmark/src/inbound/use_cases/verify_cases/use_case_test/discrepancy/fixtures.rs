//! Explicit fake-port evidence lets orchestration tests distinguish reference and candidate contracts.
use super::super::shard::fixtures::{Shape, prepared};
use crate::outbound::{
    linter::{fixtures::fake::LinterFake, interface::LinterPort},
    suite::fixtures::fake::SuiteFake,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{CoverageAccounting, PairCount, RuleCoverage, RunReport};
use ryo_benchmark_contracts::domain::requests::types::{
    ParserConfigurationName, ParserSelection, ProviderSelection, RunRequest, SearchRoots,
};
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, DiscrepancyBasis, DiscrepancyReason, FileObservation,
    MetadataDigestDeclaration, PreparedReferenceBehavior,
    PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity, RuleContextProfile, Tool,
    discrepancy_evidence::value::{
        DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
    },
};

/// Candidate fixture exposes complete execution accounting independently of its findings.
pub(crate) struct CoveredCandidate(pub LinterFake);
impl LinterPort for CoveredCandidate {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        LinterPort::prepare(&self.0, request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = LinterPort::decode(&self.0, request, output)?;
        report.coverage = CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        });
        Ok(report)
    }
}

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
            .join(crate::domain::suite::constants::BENCHMARK_TOML),
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
