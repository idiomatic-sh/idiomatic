//! Digests, expectations and admission rules of a reference discrepancy, over plain inputs.
use super::constants;
use super::errors::DiscrepancyError;
use super::types::{Contestant, IdentityText, IntentMatch};
use crate::domain::canonical_text::{
    service::digest,
    types::{Fragment, Label, Quantity},
};
use crate::domain::diagnostics::service::compare_diagnostics as compare;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::reports::types::{RunReport, SourceByteOffset};
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, DiscrepancyBasis, PreparedCase, PreparedReferenceDiscrepancy,
    ReferenceDiscrepancyIdentity, Tool, discrepancy_evidence::value::DiscrepancyEvidence,
};
use std::collections::BTreeMap;

fn offset(value: &SourceByteOffset) -> Fragment {
    Fragment::Number(Quantity(i64::try_from(value.0).unwrap_or(i64::MAX)))
}

/// Validate immutable identity spelling at the semantic conversion boundary.
pub(crate) fn identity_digest(
    value: impl Into<IdentityText>,
) -> Result<ArtifactDigest, DiscrepancyError> {
    let IdentityText(value) = value.into();
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(DiscrepancyError::IdentityNotLowercaseSha256);
    }
    Ok(ArtifactDigest(value))
}

/// Exact diagnostic multiset and input inventory; execution accounting is checked separately.
pub(crate) fn output_digest(report: &RunReport) -> ArtifactDigest {
    let mut findings = report.diagnostics.clone();
    findings.sort();
    let mut files = report
        .files
        .iter()
        .map(|file| file.0.as_ref())
        .collect::<Vec<&str>>();
    files.sort_unstable();
    let diagnostics = findings
        .iter()
        .map(|row| {
            Fragment::Object(BTreeMap::from([
                (
                    Label::from(constants::RULE),
                    Fragment::Text(Label::from(row.rule.as_ref())),
                ),
                (
                    Label::from(constants::FILE),
                    Fragment::Text(Label::from(row.file.0.as_ref())),
                ),
                (Label::from(constants::START), offset(&row.start)),
                (Label::from(constants::END), offset(&row.end)),
            ]))
        })
        .collect();
    digest(&Fragment::Object(BTreeMap::from([
        (
            Label::from(constants::FILES),
            Fragment::List(
                files
                    .into_iter()
                    .map(|file| Fragment::Text(Label::from(file)))
                    .collect(),
            ),
        ),
        (
            Label::from(constants::DIAGNOSTICS),
            Fragment::List(diagnostics),
        ),
        (Label::from(constants::SKIPPED), Fragment::List(Vec::new())),
    ])))
}

/// Distinct canonical identity for reports versus process templates.
pub(crate) fn reference_digest(expected: &ReferenceExpectation) -> ArtifactDigest {
    match expected {
        ReferenceExpectation::Diagnostics(report) => output_digest(report),
        ReferenceExpectation::ProcessFailure(process) => {
            crate::domain::process_failure::service::output_digest(process)
        }
    }
}

/// Preserve the strict intended contract unless an independently pinned reference differs.
pub(crate) fn reference_expected(case: &PreparedCase) -> Result<&RunReport, DiscrepancyError> {
    match case
        .reference_discrepancy
        .as_ref()
        .map(|item| &item.expected)
    {
        None => Ok(&case.expected),
        Some(ReferenceExpectation::Diagnostics(report)) => Ok(report),
        Some(ReferenceExpectation::ProcessFailure(_)) => {
            Err(DiscrepancyError::ProcessExpectationNotDiagnostic)
        }
    }
}

/// Raw parity remains false for the pinned reference disagreement even when intended behavior passes.
pub(crate) fn matches_intended(case: &PreparedCase, actual: &RunReport) -> IntentMatch {
    if compare(&case.expected, actual).is_ok() {
        IntentMatch::Matches
    } else {
        IntentMatch::Differs
    }
}

/// A discrepancy cannot hide unknown execution, syntax skips or missing candidate accounting.
pub(crate) fn complete(
    case: &PreparedCase,
    report: &RunReport,
    contestant: Contestant,
) -> Result<(), DiscrepancyError> {
    if case.reference_discrepancy.is_none() {
        return Ok(());
    }
    let candidate = contestant == Contestant::Candidate;
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
        return Err(DiscrepancyError::CoverageNotCompleteAndKnown);
    }
    Ok(())
}

/// The reference and intended contracts differ, are full, and carry evidence and a reason.
pub(crate) fn require_distinct(
    case: &PreparedCase,
    item: &PreparedReferenceDiscrepancy,
) -> Result<(), DiscrepancyError> {
    if case.comparison != CaseComparison::Full
        || matches!(&item.expected, ReferenceExpectation::Diagnostics(report) if compare(&case.expected, report).is_ok())
        || item.reason.trim().is_empty()
        || item.evidence.is_empty()
    {
        return Err(DiscrepancyError::ExpectationsNotDistinctFullEvidenced);
    }
    Ok(())
}

/// Approved native binaries may differ from the binary used in the original observation.
pub(crate) fn validate_pins(
    identity: &ReferenceDiscrepancyIdentity,
    profile: &Tool,
    actual: &ArtifactDigest,
    profile_digest: &ArtifactDigest,
) -> Result<(), DiscrepancyError> {
    let approved = |digest: &ArtifactDigest| profile.sha256.iter().any(|pin| pin.0 == digest.0);
    if identity.tool != profile.tool
        || identity.version != profile.version
        || !approved(&identity.observed_executable_sha256)
        || !approved(actual)
        || profile_digest != &identity.profile_sha256
    {
        return Err(DiscrepancyError::StaleToolProfileOrExecutable);
    }
    Ok(())
}

/// Every evidence row matches its declared authority and, for a language, the configured contract.
pub(crate) fn check_evidence(
    item: &PreparedReferenceDiscrepancy,
    profile: &Tool,
) -> Result<(), DiscrepancyError> {
    for row in &item.evidence {
        match (item.basis, row) {
            (
                DiscrepancyBasis::LanguageDefect,
                DiscrepancyEvidence::Language {
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
                    return Err(DiscrepancyError::EvidenceDoesNotSupportConfiguration);
                }
            }
            (
                DiscrepancyBasis::ToolContract,
                DiscrepancyEvidence::PublicToolContract { .. }
                | DiscrepancyEvidence::PriorWorkerRun { .. },
            ) => {}
            _ => return Err(DiscrepancyError::AuthorityAndEvidenceDiffer),
        }
    }
    Ok(())
}
