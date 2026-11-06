//! Admit independently evidenced reference disagreements without relaxing either contract.
use crate::application::run::constants;
use crate::domain::diagnostics::service::compare_diagnostics as compare;
use crate::outbound::{
    linter::interface::LinterPort,
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use sha2::Digest as _;
use std::path::Path;

/// Convert boundary-owned wire data into canonical case expectations before verification.
pub(super) fn attach(
    suite: &dyn SuitePort,
    rules: &Path,
    cases: &mut [PreparedCase],
) -> Result<(), Error> {
    let profile = rules
        .parent()
        .ok_or_else(|| Error::Invalid(constants::MISSING_CASE_PROFILE_BOUNDARY.into()))?;
    let tool = suite
        .profile(ArtifactPath(profile))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let wires = suite
        .case_reference_wires(ArtifactPath(profile), &tool.defaults)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let mut seen = std::collections::BTreeSet::new();
    for wire in wires {
        if !seen.insert((wire.rule.clone(), wire.name.clone())) {
            return Err(Error::Invalid(
                constants::DUPLICATE_REFERENCE_EXPECTATION_IDENTITY.into(),
            ));
        }
        let case = cases
            .iter_mut()
            .find(|case| case.rule.as_ref() == wire.rule && case.name.to_string() == wire.name)
            .ok_or_else(|| {
                Error::Invalid(constants::REFERENCE_EXPECTATION_OUTSIDE_EXECUTABLE_CAS.into())
            })?;
        if case.reference_discrepancy.is_some() {
            return Err(Error::Invalid(
                constants::DUPLICATE_CANONICAL_REFERENCE_EXPECTATION.into(),
            ));
        }
        case.reference_discrepancy = Some(convert(wire.discrepancy, wire.basis, case)?);
    }
    Ok(())
}

/// Validate immutable identity spelling at the semantic conversion boundary.
pub(super) fn digest(value: String) -> Result<ArtifactDigest, Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(Error::Invalid(
            constants::DISCREPANCY_REQUIRES_LOWERCASE_SHA256_IDENTI.into(),
        ));
    }
    Ok(ArtifactDigest(value))
}

/// Construct only complete domain expectations, without any filesystem reads.
fn convert(
    wire: crate::outbound::suite::types::discrepancy::ReferenceDiscrepancyWire,
    authority: crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire,
    case: &PreparedCase,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceDiscrepancy, Error>
{
    use crate::outbound::suite::types::discrepancy::{DiscrepancyBasisWire, EvidenceWire};
    use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
    use ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceDiscrepancy;
    use ryo_benchmark_contracts::domain::workloads::types::ReferenceDiscrepancyIdentity;
    use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence;
    use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidenceKind;
    let basis = match authority {
        DiscrepancyBasisWire::LanguageDefect => DiscrepancyBasis::LanguageDefect,
        DiscrepancyBasisWire::ToolContract => DiscrepancyBasis::ToolContract,
    };
    let identity = wire.identity;
    let process = wire.process;
    if process.is_some() && !wire.expected.is_empty() {
        return Err(Error::Invalid(
            constants::PROCESS_EXPECTATION_CANNOT_ALSO_DECLARE_REFE.into(),
        ));
    }
    let mut expected = case.expected.clone();
    expected.diagnostics = wire
        .expected
        .into_iter()
        .map(|row| {
            let file = ryo::linter::RelativePath(row.file.into_boxed_str());
            if !case.request.files.contains(&file) || row.start > row.end {
                return Err(Error::Invalid(
                    constants::INVALID_REFERENCE_EXPECTATION_SOURCE_RANGE.into(),
                ));
            }
            Ok(
                ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding {
                    rule: ryo::linter::RuleId::try_from(row.rule.as_str())
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                    file,
                    start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                        row.start,
                    ),
                    end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(row.end),
                },
            )
        })
        .collect::<Result<_, Error>>()?;
    expected.diagnostics.sort();
    if basis == DiscrepancyBasis::ToolContract && process.is_some() {
        return Err(Error::Invalid(
            constants::TOOL_CONTRACT_DIFFERENCES_REQUIRE_COMPLETE_D.into(),
        ));
    }
    let evidence = wire.evidence.into_iter().map(|item| {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(item.artifact().0))?;
        let language_spec = matches!(&item, EvidenceWire::LanguageSpec { .. });
        match (basis, item) {
            (DiscrepancyBasis::LanguageDefect, EvidenceWire::LanguageSpec { path, sha256, language_version, target_configuration, version_rationale })
            | (DiscrepancyBasis::LanguageDefect, EvidenceWire::IndependentLanguageRun { path, sha256, language_version, target_configuration, version_rationale }) => {
                if language_version.trim().is_empty() || target_configuration.trim().is_empty() || version_rationale.trim().is_empty() {
                    return Err(Error::Invalid(constants::INDEPENDENT_EVIDENCE_REQUIRES_EXPLICIT_LANGU.into()));
                }
                let kind = if language_spec { DiscrepancyEvidenceKind::LanguageSpec } else { DiscrepancyEvidenceKind::IndependentLanguageRun };
                Ok(DiscrepancyEvidence::Language { path, sha256: digest(sha256)?, kind, language_version, target_configuration, version_rationale })
            }
            (DiscrepancyBasis::ToolContract, EvidenceWire::PublicToolContract { path, sha256, source_revision, source_path, rationale }) => {
                ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(std::path::Path::new(&source_path)))?;
                if source_revision.len() != 40 || !source_revision.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) || rationale.trim().is_empty() {
                    return Err(Error::Invalid(constants::TOOL_CONTRACT_EVIDENCE_NEEDS_A_PINNED.into()));
                }
                Ok(DiscrepancyEvidence::PublicToolContract { path, sha256: digest(sha256)?, source_revision, source_path, rationale })
            }
            (DiscrepancyBasis::ToolContract, EvidenceWire::PriorWorkerRun { path, sha256, worker_sha256, control_source_sha256, control_query_sha256, control_profile_sha256, witness_case, rationale }) => {
                if rationale.trim().is_empty() || witness_case.trim().is_empty() { return Err(Error::Invalid(constants::PRIOR_WORKER_EVIDENCE_NEEDS_A_NAMED.into())); }
                Ok(DiscrepancyEvidence::PriorWorkerRun { path, sha256: digest(sha256)?, worker_sha256: digest(worker_sha256)?, control_source_sha256: digest(control_source_sha256)?, control_query_sha256: digest(control_query_sha256)?, control_profile_sha256: digest(control_profile_sha256)?, witness_case, rationale })
            }
            _ => Err(Error::Invalid(constants::DISCREPANCY_EVIDENCE_DOES_NOT_MATCH_ITS.into())),
        }
    }).collect::<Result<Vec<_>, Error>>()?;
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
        return Err(Error::Invalid(
            constants::TOOL_CONTRACT_DIFFERENCE_REQUIRES_ONE_PUBLIC.into(),
        ));
    }
    Ok(PreparedReferenceDiscrepancy {
        basis,
        expected: match process {
            Some(process) => {
                ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::ProcessFailure(
                    super::process_discrepancy::convert(process)?,
                )
            }
            None => {
                ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::Diagnostics(expected)
            }
        },
        evidence,
        reason: wire.reason,
        identity: ReferenceDiscrepancyIdentity {
            file_metadata_sha256: identity.file_metadata_sha256.map(digest).transpose()?,
            tool: identity.tool.into(),
            version: identity.version.into(),
            observed_executable_sha256: digest(identity.observed_executable_sha256)?,
            profile_sha256: digest(identity.profile_sha256)?,
            settings_sha256: digest(identity.settings_sha256)?,
            source_sha256: digest(identity.source_sha256)?,
            reference_output_sha256: digest(identity.reference_output_sha256)?,
            intended_output_sha256: digest(identity.intended_output_sha256)?,
        },
    })
}

/// Preserve the strict intended contract unless an independently pinned reference differs.
pub(super) fn reference_expected(case: &PreparedCase) -> Result<&RunReport, Error> {
    use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
    match case
        .reference_discrepancy
        .as_ref()
        .map(|item| &item.expected)
    {
        None => Ok(&case.expected),
        Some(ReferenceExpectation::Diagnostics(report)) => Ok(report),
        Some(ReferenceExpectation::ProcessFailure(_)) => Err(Error::Invalid(
            constants::PROCESS_EXPECTATION_IS_NOT_A_DIAGNOSTIC.into(),
        )),
    }
}

/// Raw parity remains false for the pinned reference disagreement even when intended behavior passes.
pub(super) fn matches_intended(case: &PreparedCase, actual: &RunReport) -> bool {
    compare(&case.expected, actual).is_ok()
}

/// Hash a canonical JSON value using the same sorted-object encoding as the reference harness.
pub(super) fn json_digest(value: &serde_json::Value) -> ArtifactDigest {
    ArtifactDigest(format!(
        "{:x}",
        sha2::Sha256::digest(value.to_string().as_bytes())
    ))
}

/// Exact diagnostic multiset and input inventory; execution accounting is checked separately.
pub(super) fn output_digest(report: &RunReport) -> ArtifactDigest {
    let mut findings = report.diagnostics.clone();
    findings.sort();
    let mut files = report
        .files
        .iter()
        .map(|file| file.0.as_ref())
        .collect::<Vec<_>>();
    files.sort_unstable();
    json_digest(&serde_json::json!({
        constants::FILES: files,
        constants::DIAGNOSTICS: findings.iter().map(|row| serde_json::json!({
            constants::RULE: row.rule.as_ref(), constants::FILE: row.file.0.as_ref(), constants::START: row.start.0, constants::END: row.end.0,
        })).collect::<Vec<_>>(),
        constants::SKIPPED: [],
    }))
}

/// A discrepancy cannot hide unknown execution, syntax skips or missing candidate accounting.
pub(super) fn complete(
    case: &PreparedCase,
    report: &RunReport,
    candidate: bool,
) -> Result<(), Error> {
    if case.reference_discrepancy.is_none() {
        return Ok(());
    }
    if (candidate
        && ryo_benchmark_contracts::domain::reports::service::require_complete_execution(report)
            .is_err())
        || !report.skipped.is_empty()
        || report.coverage.is_some_and(|coverage| {
            coverage.selected == 0
                || coverage.unknown_or_unread != 0
                || coverage.skipped != 0
                || coverage.evaluated != coverage.selected
        })
        || (candidate && report.coverage.is_none())
    {
        return Err(Error::Invalid(
            constants::REFERENCE_DISCREPANCY_REQUIRES_COMPLETE_KNOW.into(),
        ));
    }
    Ok(())
}

/// Refuse stale evidence across the selected inventory before either contestant executes.
pub(super) fn validate_all(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    rules: &Path,
    cases: &[PreparedCase],
) -> Result<Vec<Option<serde_json::Value>>, Error> {
    cases
        .iter()
        .map(|case| validate(suite, reference, rules, case))
        .collect()
}

/// Bind both expected outputs, independent evidence and the actual approved reference invocation.
pub(super) fn validate(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    rules: &Path,
    case: &PreparedCase,
) -> Result<Option<serde_json::Value>, Error> {
    let Some(item) = &case.reference_discrepancy else {
        return Ok(None);
    };
    if case.comparison != CaseComparison::Full
        || matches!(&item.expected, ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::Diagnostics(report) if compare(&case.expected, report).is_ok())
        || item.reason.trim().is_empty()
        || item.evidence.is_empty()
    {
        return Err(Error::Invalid(
            constants::REFERENCE_DISCREPANCY_REQUIRES_DISTINCT_FULL.into(),
        ));
    }
    let profile_root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(constants::MISSING_PROFILE_ROOT.into()))?;
    let profile = suite.profile(ArtifactPath(profile_root))?;
    let identity = &item.identity;
    let invocation = reference.prepare(&case.request)?;
    suite.verify_tool(ArtifactPath(&invocation.executable), &profile)?;
    let actual_executable = suite.digest(ArtifactPath(&invocation.executable))?;
    validate_pins(
        identity,
        &profile,
        &actual_executable,
        &suite.digest(ArtifactPath(&profile_root.join(constants::BENCHMARK_TOML)))?,
    )?;
    let settings = super::invocation_identity::invocation_settings(&invocation, case, &profile)?;
    let files = crate::domain::suite::service::materialized_sources(&case.request);
    let evidence = item
        .evidence
        .iter()
        .map(|row| {
            use crate::outbound::suite::types::discrepancy::{
                EvidenceArtifact, EvidenceRequirement,
            };
            let requirement = match row {
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::Language { .. } => {
                    EvidenceRequirement::Language
                }
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::PublicToolContract {
                    source_revision,
                    source_path,
                    ..
                } => EvidenceRequirement::PublicToolContract {
                    source_revision: source_revision.clone(),
                    source_path: source_path.clone(),
                },
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::PriorWorkerRun {
                    worker_sha256,
                    control_source_sha256,
                    control_query_sha256,
                    control_profile_sha256,
                    witness_case,
                    ..
                } => EvidenceRequirement::PriorWorkerRun {
                    worker_sha256: worker_sha256.0.clone(),
                    control_source_sha256: control_source_sha256.0.clone(),
                    control_query_sha256: control_query_sha256.0.clone(),
                    control_profile_sha256: control_profile_sha256.0.clone(),
                    witness_case: witness_case.clone(),
                },
            };
            EvidenceArtifact {
                path: row.artifact().0.clone(),
                sha256: row.artifact().1.0.clone(),
                requirement,
            }
        })
        .collect::<Vec<_>>();
    for row in &item.evidence {
        match (item.basis, row) {
            (
                ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::Language {
                    language_version,
                    target_configuration,
                    version_rationale,
                    ..
                },
            ) => {
                if language_version.trim().is_empty()
                    || version_rationale.trim().is_empty()
                    || profile.parser_configuration.as_deref()
                        != Some(target_configuration.as_str())
                {
                    return Err(Error::Invalid(
                        constants::INDEPENDENT_EVIDENCE_DOES_NOT_SUPPORT_CONFIG.into(),
                    ));
                }
            }
            (
                ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::ToolContract,
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::PublicToolContract { .. }
                | ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::PriorWorkerRun { .. },
            ) => {}
            _ => {
                return Err(Error::Invalid(
                    constants::DISCREPANCY_AUTHORITY_AND_EVIDENCE_DIFFER.into(),
                ));
            }
        }
    }
    let metadata_files = case
        .request
        .file_metadata
        .as_ref()
        .map(|values| {
            values
                .keys()
                .map(|file| std::path::PathBuf::from(file.0.as_ref()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    suite
        .verify_discrepancy_artifacts(crate::outbound::suite::types::discrepancy::CaseArtifacts {
            profile: profile_root,
            root: &case.request.root,
            files: &files,
            file_metadata_files: &metadata_files,
            source_sha256: &identity.source_sha256.0,
            file_metadata_sha256: identity
                .file_metadata_sha256
                .as_ref()
                .map(|value| value.0.as_str()),
            evidence: &evidence,
        })
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((case.request.file_metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })))?
        != identity.file_metadata_sha256
        || settings != identity.settings_sha256
        || output_digest(&case.expected) != identity.intended_output_sha256
        || reference_digest(&item.expected) != identity.reference_output_sha256
    {
        return Err(Error::Invalid(
            constants::STALE_REFERENCE_DISCREPANCY_SETTINGS_SOURCE.into(),
        ));
    }
    if super::process_discrepancy::expected(case).is_some() {
        super::process_discrepancy::validate_archives(
            suite,
            profile_root,
            &profile,
            case,
            identity,
            &actual_executable,
        )?;
    }
    let mut validation = serde_json::json!({
        constants::DISCREPANCY_BASIS: match item.basis { ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect => constants::LANGUAGE_DEFECT, ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::ToolContract => constants::TOOL_CONTRACT_DIFFERENCE },
        constants::PROCESS_OBSERVATION_GROUP_SHA256: super::process_discrepancy::expected(case).map(|process|super::process_discrepancy::group_digest(process,&actual_executable.0)).transpose()?.map(|value|value.0),
        constants::REFERENCE_EXPECTATION_KIND: if super::process_discrepancy::expected(case).is_some() { constants::PROCESS_FAILURE } else { constants::DIAGNOSTICS },
        constants::SETTINGS_SHA256: settings.0,
        constants::ACTUAL_EXECUTABLE_SHA256: actual_executable.0,
        constants::PROFILE_SHA256: identity.profile_sha256.0,
        constants::SOURCE_SHA256: identity.source_sha256.0,
        constants::REFERENCE_OUTPUT_SHA256: identity.reference_output_sha256.0,
        constants::INTENDED_OUTPUT_SHA256: identity.intended_output_sha256.0,
    });
    if let Some(digest) = &identity.file_metadata_sha256 {
        validation[constants::FILE_METADATA_SHA256] = serde_json::json!(digest.0);
    }
    Ok(Some(validation))
}

/// Approved native binaries may differ from the binary used in the original observation.
fn validate_pins(
    identity: &ryo_benchmark_contracts::domain::workloads::types::ReferenceDiscrepancyIdentity,
    profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    actual: &ArtifactDigest,
    profile_digest: &ArtifactDigest,
) -> Result<(), Error> {
    let approved = |digest: &ArtifactDigest| profile.sha256.iter().any(|pin| pin.0 == digest.0);
    if identity.tool != profile.tool
        || identity.version != profile.version
        || !approved(&identity.observed_executable_sha256)
        || !approved(actual)
        || profile_digest != &identity.profile_sha256
    {
        return Err(Error::Invalid(
            constants::STALE_REFERENCE_DISCREPANCY_TOOL_PROFILE_OR.into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "discrepancy_test.rs"]
mod tests;

/// Distinct canonical identity for reports versus process templates.
fn reference_digest(
    expected: &ryo_benchmark_contracts::domain::process::types::ReferenceExpectation,
) -> ArtifactDigest {
    use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
    match expected {
        ReferenceExpectation::Diagnostics(report) => output_digest(report),
        ReferenceExpectation::ProcessFailure(process) => {
            super::process_discrepancy::output_digest(process)
        }
    }
}
