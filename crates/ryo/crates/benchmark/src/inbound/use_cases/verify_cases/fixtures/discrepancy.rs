//! A documented reference difference keeps the intended contract and the pinned reference apart.
use crate::outbound::linter::fixtures::fake::LinterFake;
use crate::outbound::linter::interface::LinterPort as _;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use crate::outbound::suite::types::CaseClaim;
use crate::outbound::suite::types::ClaimedDiscrepancy;
use crate::outbound::suite::types::ClaimedEvidence;
use crate::outbound::suite::types::ClaimedIdentity;
use crate::outbound::suite::types::ClaimedMetadata;
use crate::outbound::suite::types::ClaimedProcess;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::documents::types::text::value::CaseName;
use ryo_benchmark_contracts::domain::documents::types::text::value::RuleName;
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName;
use ryo_benchmark_contracts::domain::requests::types::ParserSelection;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyReason;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::LanguageVersion;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::ParserTarget;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::Rationale;
use super::shard::Shape;
use super::shard::prepared;

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
    let mut profile = suite.profile.take().expect("authored profile");
    profile.project_context_mode = ProjectContextMode::Isolated;
    profile.sha256 = vec![binary_hash.0.clone().into()];
    profile.defaults = vec![case.rule.clone()];
    profile.parser_configuration =
        ParserSelection::Named(ParserConfigurationName("python-target=3.10".into()));
    let invocation = LinterFake::default().prepare(&case.request).expect("fixture invocation");
    let settings = crate::domain::canonical_text::service::text_digest(
        &crate::application::discrepancy_validation::service::settings_text(
            &LinterFake::default(),
            &invocation,
            case,
            &profile,
        )
        .expect("fixture settings"),
    );
    let sha = |digest: &ArtifactDigest| Sha256Text(digest.0.clone());
    let evidence_digest = Sha256Text("d".repeat(64));
    let claim = CaseClaim {
        rule: RuleName(case.rule.to_string()),
        name: CaseName(case.name.to_string()),
        basis: DiscrepancyBasis::LanguageDefect,
        discrepancy: ClaimedDiscrepancy {
            process: ClaimedProcess::Absent,
            expected: Vec::new(),
            identity: ClaimedIdentity {
                file_metadata_sha256: ClaimedMetadata::Undeclared,
                tool: "fixture".into(),
                version: "1".into(),
                observed_executable_sha256: sha(&binary_hash),
                profile_sha256: sha(&profile_hash),
                settings_sha256: sha(&settings),
                source_sha256: Sha256Text("c".repeat(64)),
                reference_output_sha256: sha(&crate::domain::discrepancy::service::output_digest(&reference)),
                intended_output_sha256: sha(&crate::domain::discrepancy::service::output_digest(&case.expected)),
            },
            evidence: vec![ClaimedEvidence::IndependentLanguageRun {
                path: "evidence.json".into(),
                sha256: evidence_digest,
                language_version: LanguageVersion("3.10".into()),
                target_configuration: ParserTarget("python-target=3.10".into()),
                version_rationale: Rationale("exact target independently evaluated".into()),
            }],
            reason: DiscrepancyReason(
                "independent language contract differs from the pinned reference".into(),
            ),
        },
    };
    suite.reference_claims.push(claim);
    suite.profile = Some(profile);
    let manifest = std::path::Path::new("/profile")
        .join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
    suite.artifact_digests.insert(manifest, profile_hash);
    suite.artifact_digests.insert("fixture".into(), binary_hash);
    suite.discrepancy_artifacts_valid = true;
    suite
}
