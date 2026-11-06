//! Convert boundary-owned wire data into canonical case expectations before verification.
use super::errors::DiscrepancyAttachmentError as Failure;
use crate::domain::discrepancy::service::identity_digest;
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{
        ArtifactPath,
        discrepancy::{DiscrepancyBasisWire, EvidenceWire, ReferenceDiscrepancyWire},
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::process::types::{ProcessFailure, ReferenceExpectation};
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};
use ryo_benchmark_contracts::domain::workloads::types::{
    DiscrepancyBasis, PreparedCase, PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity,
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
    let wires = suite
        .case_reference_wires(ArtifactPath(profile), &tool.defaults)
        .map_err(refused)?;
    let mut seen = std::collections::BTreeSet::new();
    for wire in wires {
        if !seen.insert((wire.rule.clone(), wire.name.clone())) {
            return Err(Failure::DuplicateExpectationIdentity);
        }
        let case = cases
            .iter_mut()
            .find(|case| case.rule.as_ref() == wire.rule && case.name.to_string() == wire.name)
            .ok_or(Failure::ExpectationOutsideInventory)?;
        if case.reference_discrepancy.is_some() {
            return Err(Failure::DuplicateCanonicalExpectation);
        }
        case.reference_discrepancy = Some(convert(wire.discrepancy, wire.basis, case)?);
    }
    Ok(())
}

fn finding(
    row: crate::outbound::suite::types::discrepancy::ReferenceFindingWire,
    case: &PreparedCase,
) -> Result<BenchmarkFinding, Failure> {
    let file = ryo::linter::RelativePath(row.file.into_boxed_str());
    if !case.request.files.contains(&file) || row.start > row.end {
        return Err(Failure::InvalidSourceRange);
    }
    Ok(BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(row.rule.as_str()).map_err(refused)?,
        file,
        start: SourceByteOffset(row.start),
        end: SourceByteOffset(row.end),
    })
}

fn convert_evidence(
    item: EvidenceWire,
    basis: DiscrepancyBasis,
) -> Result<DiscrepancyEvidence, Failure> {
    let _artifact = RelativeArtifactPath::try_from(Path::new(item.artifact().0))?;
    let language_spec = matches!(&item, EvidenceWire::LanguageSpec { .. });
    match (basis, item) {
        (
            DiscrepancyBasis::LanguageDefect,
            EvidenceWire::LanguageSpec {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            }
            | EvidenceWire::IndependentLanguageRun {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            },
        ) => {
            if language_version.trim().is_empty()
                || target_configuration.trim().is_empty()
                || version_rationale.trim().is_empty()
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
                sha256: identity_digest(sha256)?,
                kind,
                language_version,
                target_configuration,
                version_rationale,
            })
        }
        (
            DiscrepancyBasis::ToolContract,
            EvidenceWire::PublicToolContract {
                path,
                sha256,
                source_revision,
                source_path,
                rationale,
            },
        ) => {
            let _source = RelativeArtifactPath::try_from(Path::new(&source_path))?;
            if source_revision.len() != 40
                || !source_revision
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || rationale.trim().is_empty()
            {
                return Err(Failure::ToolContractEvidenceNeedsSource);
            }
            Ok(DiscrepancyEvidence::PublicToolContract {
                path,
                sha256: identity_digest(sha256)?,
                source_revision,
                source_path,
                rationale,
            })
        }
        (
            DiscrepancyBasis::ToolContract,
            EvidenceWire::PriorWorkerRun {
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
            if rationale.trim().is_empty() || witness_case.trim().is_empty() {
                return Err(Failure::PriorWorkerEvidenceNeedsControl);
            }
            Ok(DiscrepancyEvidence::PriorWorkerRun {
                path,
                sha256: identity_digest(sha256)?,
                worker_sha256: identity_digest(worker_sha256)?,
                control_source_sha256: identity_digest(control_source_sha256)?,
                control_query_sha256: identity_digest(control_query_sha256)?,
                control_profile_sha256: identity_digest(control_profile_sha256)?,
                witness_case,
                rationale,
            })
        }
        _ => Err(Failure::EvidenceDoesNotMatchAuthority),
    }
}

/// Construct only complete domain expectations, without any filesystem reads.
fn convert(
    wire: ReferenceDiscrepancyWire,
    authority: DiscrepancyBasisWire,
    case: &PreparedCase,
) -> Result<PreparedReferenceDiscrepancy, Failure> {
    let basis = match authority {
        DiscrepancyBasisWire::LanguageDefect => DiscrepancyBasis::LanguageDefect,
        DiscrepancyBasisWire::ToolContract => DiscrepancyBasis::ToolContract,
    };
    let identity = wire.identity;
    let process = wire.process;
    if process.is_some() && !wire.expected.is_empty() {
        return Err(Failure::ProcessAlsoDeclaresDiagnostics);
    }
    let mut expected = case.expected.clone();
    expected.diagnostics = wire
        .expected
        .into_iter()
        .map(|row| finding(row, case))
        .collect::<Result<_, Failure>>()?;
    expected.diagnostics.sort();
    if basis == DiscrepancyBasis::ToolContract && process.is_some() {
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
            Some(process) => {
                ReferenceExpectation::ProcessFailure(ProcessFailure::try_from(process)?)
            }
            None => ReferenceExpectation::Diagnostics(expected),
        },
        evidence,
        reason: wire.reason,
        identity: ReferenceDiscrepancyIdentity {
            file_metadata_sha256: identity
                .file_metadata_sha256
                .map(identity_digest)
                .transpose()?,
            tool: identity.tool.into(),
            version: identity.version.into(),
            observed_executable_sha256: identity_digest(identity.observed_executable_sha256)?,
            profile_sha256: identity_digest(identity.profile_sha256)?,
            settings_sha256: identity_digest(identity.settings_sha256)?,
            source_sha256: identity_digest(identity.source_sha256)?,
            reference_output_sha256: identity_digest(identity.reference_output_sha256)?,
            intended_output_sha256: identity_digest(identity.intended_output_sha256)?,
        },
    })
}
