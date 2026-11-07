//! Pure validation shared by artifact auditing and case execution.
use super::types::{CaseOwnership, ContextAbsence};
use crate::domain::suite::constants;
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{CoverageAccounting, ProjectContext};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    Cases, ContextDeclaration, InventoryDeclaration, ReferenceBehavior, RequestedFiles,
};
use std::path::{Component, Path};

/// Materialized source identity includes context files, independently of requested diagnostics.
#[must_use]
pub fn materialized_sources(
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
) -> Vec<std::path::PathBuf> {
    let listed = match &request.staged_sources {
        StagedSources::Listed(files) => Some(files),
        StagedSources::Unlisted => None,
    };
    listed
        .unwrap_or(&request.files)
        .iter()
        .filter(|file| {
            listed.is_some()
                || match &request.context_manifest {
                    ContextSeal::Sealed(manifest) => {
                        declares_missing(manifest, file) == ContextAbsence::Unnamed
                    }
                    ContextSeal::Unsealed => true,
                }
        })
        .map(|file| std::path::PathBuf::from(file.0.as_ref()))
        .collect()
}

/// Only explicit default definitions own case evidence; supporting definitions supply executable queries.
#[must_use]
pub fn owns_cases(
    tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    rule: &ryo::linter::RuleId,
) -> CaseOwnership {
    if tool.defaults.contains(rule) {
        CaseOwnership::Owns
    } else {
        CaseOwnership::Supports
    }
}

/// Require a nonempty complete implementation inventory.
///
/// # Errors
/// Refuses an empty inventory or any missing default implementation.
pub fn require_complete(audit: &Audit) -> Result<(), Error> {
    if audit.implemented.is_empty() && audit.missing.is_empty() {
        return Err(Error::Invalid(super::constants::EMPTY_AUDIT.into()));
    }
    if audit.missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "default coverage incomplete: {} missing rules: {}",
            audit.missing.len(),
            audit
                .missing
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(super::constants::RULE_SEPARATOR)
        )))
    }
}

/// Every default is either authored or explicitly deferred; no gap becomes a match.
///
/// # Errors
/// Refuses an empty implementation subset, invalid deferrals or unaccounted defaults.
pub fn require_accounted(
    audit: &Audit,
    tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
) -> Result<(), Error> {
    if audit.implemented.is_empty() {
        return Err(Error::Invalid(super::constants::EMPTY_SUBSET.into()));
    }
    for (rule, issue) in &tool.deferred {
        if !audit.missing.contains(rule) || issue.0.is_empty() {
            return Err(Error::Invalid(format!(
                "invalid default deferral: {rule} -> {issue}"
            )));
        }
    }
    let unaccounted: Vec<_> = audit
        .missing
        .iter()
        .filter(|rule| !tool.deferred.contains_key(*rule))
        .collect();
    if !unaccounted.is_empty() {
        return Err(Error::Invalid(format!(
            "defaults lack implementation or explicit deferral: {unaccounted:?}"
        )));
    }
    Ok(())
}

/// Require independent positive/negative cases for the enclosing rule with valid source spans and names.
///
/// # Errors
/// Full-report cases are positive only when they contain the enclosing rule's finding; unrelated diagnostics do not change polarity.
/// Refuses missing positive or negative expectations and invalid case sources.
pub fn validate_cases(rule: &ryo::linter::RuleId, cases: &Cases) -> Result<(), Error> {
    let has_rule_finding = |case: &ryo_benchmark_contracts::domain::workloads::types::Case| {
        match case.comparison {
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Rule => !case.expected.is_empty(),
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full => case.expected.iter().any(|finding| {
            matches!(&finding.identity, ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Explicit(found) if found == rule)
        }),
    }
    };
    if !cases.cases.iter().any(&has_rule_finding) || cases.cases.iter().all(&has_rule_finding) {
        return Err(Error::Invalid(super::constants::MISSING_POLARITY.into()));
    }
    validate_case_sources(cases)
}

/// Validate case identity, source coverage and byte ranges without asserting a positive finding.
///
/// # Errors
/// Refuses empty cases, duplicate names, unsafe paths or spans outside supplied sources.
pub fn validate_case_sources(cases: &Cases) -> Result<(), Error> {
    if cases.cases.is_empty() {
        return Err(Error::Invalid(super::constants::EMPTY_CASES.into()));
    }
    let mut names = std::collections::BTreeSet::new();
    for case in &cases.cases {
        validate_selected_rules(&case.selected_rules)?;
        if (case.files.is_empty()
            && match &case.requested_files {
                RequestedFiles::Legacy => true,
                RequestedFiles::Explicit(requested) => requested.is_empty(),
            })
            || case.name.0.trim().is_empty()
            || !names.insert(&case.name)
        {
            return Err(Error::Invalid(
                super::constants::INVALID_CASE_IDENTITY.into(),
            ));
        }
        for name in case.files.keys() {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(Path::new(name.0.as_ref()))?;
        }
        if let RequestedFiles::Explicit(requested) = &case.requested_files {
            let selected: std::collections::BTreeSet<_> = requested.iter().collect();
            if requested.is_empty() || selected.len() != requested.len() {
                return Err(Error::Invalid(
                    constants::EXPLICIT_SOURCE_INVENTORY_MUST_NAME_EACH.into(),
                ));
            }
            if let ContextDeclaration::Declared(context) = &case.context {
                if case.files.keys().any(|path| !selected.contains(path)) {
                    return Err(Error::Invalid(
                        constants::SEALED_DISK_CONTEXT_MUST_SELECT_EVERY.into(),
                    ));
                }
                for path in requested {
                    ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(Path::new(path.0.as_ref()))?;
                    if !case.files.contains_key(path)
                        && context_missing(&context.logical_root, &context.missing, path)
                            == ContextAbsence::Unnamed
                    {
                        return Err(Error::Invalid(
                            constants::REQUESTED_UNAVAILABLE_SOURCE_LACKS_A_CONTEXT.into(),
                        ));
                    }
                }
            } else {
                if requested.iter().any(|path| !case.files.contains_key(path)) {
                    return Err(Error::Invalid(
                        constants::GIVEN_SOURCE_SELECTION_LACKS_A_STAGED.into(),
                    ));
                }
                for path in case.files.keys().filter(|path| !selected.contains(path)) {
                    let InventoryDeclaration::Declared(inventory) = &case.given_project_inventory
                    else {
                        return Err(Error::Invalid(
                            constants::UNSELECTED_GIVEN_SOURCE_REQUIRES_AN_EXPLICIT.into(),
                        ));
                    };
                    if !inventory.entries.iter().any(|entry| {
                        entry.path == *path && entry.state == ryo::linter::ProjectEntryState::Regular
                    }) {
                        return Err(Error::Invalid(
                            constants::UNSELECTED_GIVEN_SOURCE_LACKS_A_MATCHING.into(),
                        ));
                    }
                }
            }
        }
        if let FileMetadataDeclaration::Declared(values) = &case.file_metadata {
            for name in values.keys() {
                ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(Path::new(name.0.as_ref()))?;
                if !case.files.contains_key(name) {
                    return Err(Error::Invalid(
                        constants::FILE_METADATA_NAMES_AN_UNDECLARED_SOURCE.into(),
                    ));
                }
            }
        }
        if matches!(case.reference_discrepancy, ReferenceBehavior::Differs(_))
            && case.comparison
                != ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full
        {
            return Err(Error::Invalid(
                constants::REFERENCE_DISCREPANCIES_REQUIRE_FULL_COMPARI.into(),
            ));
        }
        let reference_expected = match &case.reference_discrepancy {
            ReferenceBehavior::Agrees => &[][..],
            ReferenceBehavior::Differs(item) => item.expected.as_slice(),
        };
        for expected in case.expected.iter().chain(reference_expected) {
            if case.comparison
                == ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full
                && matches!(
                    expected.identity,
                    ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Parent
                )
            {
                return Err(Error::Invalid(
                    super::constants::MISSING_DIAGNOSTIC_IDENTITY.into(),
                ));
            }
            let source = match case.files.get(&expected.file) {
                Some(source) => source,
                None if matches!(&case.requested_files, RequestedFiles::Explicit(paths) if paths.contains(&expected.file))
                    && expected.start.0 == 0
                    && expected.end.0 == 0 =>
                {
                    continue;
                }
                None => return Err(Error::Invalid(super::constants::MISSING_SOURCE.into())),
            };
            if expected.start > expected.end
                || !source.0.is_char_boundary(expected.start.0)
                || !source.0.is_char_boundary(expected.end.0)
            {
                return Err(Error::Invalid(
                    super::constants::INVALID_SOURCE_RANGE.into(),
                ));
            }
        }
    }
    Ok(())
}

/// Whether a context declaration names this logical input as absent. This is only membership:
/// application preparation must independently validate the physical context before execution.
#[must_use]
pub fn declares_missing(
    manifest: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
    file: &ryo::linter::RelativePath,
) -> ContextAbsence {
    context_missing(&manifest.logical_root, &manifest.missing, file)
}

/// Match logical input paths using the same harmless `.` normalization as context materialization.
fn context_missing(
    root: &Path,
    missing: &[std::path::PathBuf],
    file: &ryo::linter::RelativePath,
) -> ContextAbsence {
    let input = context_input_path(root, file);
    if missing.iter().any(|path| {
        path.components()
            .filter(|part| *part != Component::CurDir)
            .eq(input.components().filter(|part| *part != Component::CurDir))
    }) {
        ContextAbsence::Sealed
    } else {
        ContextAbsence::Unnamed
    }
}

/// A requested logical input's boundary-relative path, consistent with context materialization.
pub(crate) fn context_input_path(
    root: &Path,
    file: &ryo::linter::RelativePath,
) -> std::path::PathBuf {
    root.join(file.0.as_ref())
        .components()
        .filter(|part| *part != Component::CurDir)
        .collect()
}

/// Admit only explicit, nonempty, duplicate-free cohorts. A frozen selection preserves the historical profile selection.
///
/// # Errors
/// Refuses an explicit cohort that is empty or repeats a rule.
pub fn validate_selected_rules(rules: &CohortSelection) -> Result<(), Error> {
    if let CohortSelection::Explicit(rules) = rules {
        let unique: std::collections::BTreeSet<_> = rules.iter().collect();
        if rules.is_empty() || unique.len() != rules.len() {
            return Err(Error::Invalid(
                constants::SELECTED_RULES_MUST_BE_NONEMPTY_AND.into(),
            ));
        }
    }
    Ok(())
}

/// Bind raw artifact observations to the exact declared inventory before issuing an audit.
/// # Errors
/// Refuses duplicate, overlapping, missing or undeclared identities.
pub(crate) fn audited(
    tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    observed: ryo_benchmark_contracts::domain::reports::types::AuthoredInventory,
) -> Result<crate::domain::suite::types::audit::value::Audit, Error> {
    let defaults = tool
        .defaults
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let implemented = observed
        .implemented
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let missing = observed
        .missing
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if defaults.is_empty()
        || defaults.len() != tool.defaults.len()
        || implemented.len() != observed.implemented.len()
        || missing.len() != observed.missing.len()
        || !implemented.is_disjoint(&missing)
        || implemented
            .union(&missing)
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            != defaults
    {
        return Err(Error::Invalid(
            constants::ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE.into(),
        ));
    }
    Ok(crate::domain::suite::types::audit::value::Audit {
        implemented: observed.implemented,
        missing: observed.missing,
    })
}

/// Preserve every authored diagnostic identity and the independently expected multiset.
pub(crate) fn expected_report(
    rule: &ryo::linter::RuleId,
    files: Vec<ryo::linter::RelativePath>,
    expected: Vec<ryo_benchmark_contracts::domain::workloads::types::Expected>,
) -> ryo_benchmark_contracts::domain::reports::types::RunReport {
    let mut report = ryo_benchmark_contracts::domain::reports::types::RunReport {
        execution_metadata:
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Unknown,
        project_context: ProjectContext::NotRequested,
        files,
        skipped: Vec::new(),
        coverage: CoverageAccounting::Unaccounted,
        diagnostics: expected
            .into_iter()
            .map(
                |finding| ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding {
                    rule: match finding.identity {
                        ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Parent => rule.clone(),
                        ryo_benchmark_contracts::domain::workloads::types::ExpectedRule::Explicit(value) => value,
                    },
                    file: finding.file,
                    start: finding.start,
                    end: finding.end,
                },
            )
            .collect(),
    };
    report.diagnostics.sort();
    report
}
