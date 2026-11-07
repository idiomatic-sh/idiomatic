//! Convert boundary-owned wire data into canonical case expectations before verification.
use super::errors::DiscrepancyAttachmentError as Failure;
use crate::domain::discrepancy::service::identity_digest;
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{
        ArtifactPath, ClaimedDiscrepancy, ClaimedEvidence, ClaimedFinding, ClaimedMetadata,
        ClaimedProcess,
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::workloads::types::{
    DiscrepancyBasis, MetadataDigestDeclaration, PreparedCase, PreparedReferenceBehavior,
    PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity,
    discrepancy_evidence::value::{DiscrepancyEvidence, DiscrepancyEvidenceKind},
};
use std::path::Path;

fn refused(error: impl std::fmt::Display) -> Failure {
    Failure::Refused(RefusalText(error.to_string()))
}

/// Convert boundary-owned wire data into canonical case expectations before verification.
pub(crate) fn attach(
    suite: &dyn SuitePort,
    rules: &Path,
    cases: &mut [PreparedCase],
) -> Result<(), Failure> {
    let profile = rules.parent().ok_or(Failure::MissingCaseProfileBoundary)?;
    let tool = suite.profile(ArtifactPath(profile)).map_err(refused)?;
    let claims = suite
        .case_claims(ArtifactPath(profile), &tool.defaults)
        .map_err(refused)?;
    let mut seen = std::collections::BTreeSet::new();
    for claim in claims {
        if !seen.insert((claim.rule.0.clone(), claim.name.0.clone())) {
            return Err(Failure::DuplicateExpectationIdentity);
        }
        let case = cases
            .iter_mut()
            .find(|case| {
                case.rule.as_ref() == claim.rule.0 && case.name.to_string() == claim.name.0
            })
            .ok_or(Failure::ExpectationOutsideInventory)?;
        if !matches!(case.reference_discrepancy, PreparedReferenceBehavior::Agrees) {
            return Err(Failure::DuplicateCanonicalExpectation);
        }
        case.reference_discrepancy =
            PreparedReferenceBehavior::Differs(convert(claim.discrepancy, claim.basis, case)?);
    }
    Ok(())
}

fn finding(row: ClaimedFinding, case: &PreparedCase) -> Result<BenchmarkFinding, Failure> {
    let file = ryo::linter::RelativePath(row.file.0.into_boxed_str());
    if !case.request.files.contains(&file) || row.start > row.end {
        return Err(Failure::InvalidSourceRange);
    }
    Ok(BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(row.rule.0.as_str()).map_err(refused)?,
        file,
        start: row.start,
        end: row.end,
    })
}

/// The profile-relative artifact every kind of claimed evidence names.
fn artifact_path(item: &ClaimedEvidence) -> &Path {
    let (ClaimedEvidence::LanguageSpec { path, .. }
    | ClaimedEvidence::IndependentLanguageRun { path, .. }
    | ClaimedEvidence::PublicToolContract { path, .. }
    | ClaimedEvidence::PriorWorkerRun { path, .. }) = item;
    path
}

fn convert_evidence(
    item: ClaimedEvidence,
    basis: DiscrepancyBasis,
) -> Result<DiscrepancyEvidence, Failure> {
    let _artifact = RelativeArtifactPath::try_from(artifact_path(&item))?;
    let language_spec = matches!(&item, ClaimedEvidence::LanguageSpec { .. });
    match (basis, item) {
        (
            DiscrepancyBasis::LanguageDefect,
            ClaimedEvidence::LanguageSpec {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            }
            | ClaimedEvidence::IndependentLanguageRun {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            },
        ) => {
            if language_version.0.trim().is_empty()
                || target_configuration.0.trim().is_empty()
                || version_rationale.0.trim().is_empty()
            {
                return Err(Failure::LanguageEvidenceNeedsContract);
            }
            let kind = if language_spec {
                DiscrepancyEvidenceKind::LanguageSpec
            } else {
                DiscrepancyEvidenceKind::IndependentLanguageRun
            };
            Ok(DiscrepancyEvidence::Language {
                path,
                sha256: identity_digest(sha256.0)?,
                kind,
                language_version,
                target_configuration,
                version_rationale,
            })
        }
        (
            DiscrepancyBasis::ToolContract,
            ClaimedEvidence::PublicToolContract {
                path,
                sha256,
                source_revision,
                source_path,
                rationale,
            },
        ) => {
            let _source = RelativeArtifactPath::try_from(Path::new(&source_path.0))?;
            if source_revision.0.len() != 40
                || !source_revision
                    .0
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || rationale.0.trim().is_empty()
            {
                return Err(Failure::ToolContractEvidenceNeedsSource);
            }
            Ok(DiscrepancyEvidence::PublicToolContract {
                path,
                sha256: identity_digest(sha256.0)?,
                source_revision,
                source_path,
                rationale,
            })
        }
        (
            DiscrepancyBasis::ToolContract,
            ClaimedEvidence::PriorWorkerRun {
                path,
                sha256,
                worker_sha256,
                control_source_sha256,
                control_query_sha256,
                control_profile_sha256,
                witness_case,
                rationale,
            },
        ) => {
            if rationale.0.trim().is_empty() || witness_case.0.trim().is_empty() {
                return Err(Failure::PriorWorkerEvidenceNeedsControl);
            }
            Ok(DiscrepancyEvidence::PriorWorkerRun {
                path,
                sha256: identity_digest(sha256.0)?,
                worker_sha256: identity_digest(worker_sha256.0)?,
                control_source_sha256: identity_digest(control_source_sha256.0)?,
                control_query_sha256: identity_digest(control_query_sha256.0)?,
                control_profile_sha256: identity_digest(control_profile_sha256.0)?,
                witness_case,
                rationale,
            })
        }
        _ => Err(Failure::EvidenceDoesNotMatchAuthority),
    }
}

/// Construct only complete domain expectations, without any filesystem reads.
fn convert(
    wire: ClaimedDiscrepancy,
    basis: DiscrepancyBasis,
    case: &PreparedCase,
) -> Result<PreparedReferenceDiscrepancy, Failure> {
    let identity = wire.identity;
    let process = wire.process;
    let has_process = matches!(process, ClaimedProcess::Failure(_));
    if has_process && !wire.expected.is_empty() {
        return Err(Failure::ProcessAlsoDeclaresDiagnostics);
    }
    let mut expected = case.expected.clone();
    expected.diagnostics = wire
        .expected
        .into_iter()
        .map(|row| finding(row, case))
        .collect::<Result<_, Failure>>()?;
    expected.diagnostics.sort();
    if basis == DiscrepancyBasis::ToolContract && has_process {
        return Err(Failure::ToolContractNeedsDiagnostics);
    }
    let evidence = wire
        .evidence
        .into_iter()
        .map(|item| convert_evidence(item, basis))
        .collect::<Result<Vec<_>, Failure>>()?;
    if basis == DiscrepancyBasis::ToolContract
        && (evidence.len() != 2
            || evidence
                .iter()
                .filter(|item| matches!(item, DiscrepancyEvidence::PublicToolContract { .. }))
                .count()
                != 1
            || evidence
                .iter()
                .filter(|item| matches!(item, DiscrepancyEvidence::PriorWorkerRun { .. }))
                .count()
                != 1)
    {
        return Err(Failure::ToolContractNeedsTwoEvidenceRows);
    }
    Ok(PreparedReferenceDiscrepancy {
        basis,
        expected: match process {
            ClaimedProcess::Failure(process) => ReferenceExpectation::ProcessFailure(process),
            ClaimedProcess::Absent => ReferenceExpectation::Diagnostics(expected),
        },
        evidence,
        reason: wire.reason,
        identity: ReferenceDiscrepancyIdentity {
            file_metadata_sha256: match identity.file_metadata_sha256 {
                ClaimedMetadata::Declared(digest) => {
                    MetadataDigestDeclaration::Declared(identity_digest(digest.0)?)
                }
                ClaimedMetadata::Undeclared => MetadataDigestDeclaration::Undeclared,
            },
            tool: identity.tool,
            version: identity.version,
            observed_executable_sha256: identity_digest(identity.observed_executable_sha256.0)?,
            profile_sha256: identity_digest(identity.profile_sha256.0)?,
            settings_sha256: identity_digest(identity.settings_sha256.0)?,
            source_sha256: identity_digest(identity.source_sha256.0)?,
            reference_output_sha256: identity_digest(identity.reference_output_sha256.0)?,
            intended_output_sha256: identity_digest(identity.intended_output_sha256.0)?,
        },
    })
}
