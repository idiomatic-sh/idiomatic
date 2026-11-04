//! Pure validation shared by artifact auditing and case execution.
use super::types::{Cases, audit::value::Audit};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use std::path::{Component, Path};

/// Only explicit default definitions own case evidence; supporting definitions supply executable queries.
#[must_use]
pub fn owns_cases(tool: &super::types::Tool, rule: &ryo::linter::RuleId) -> bool {
    tool.defaults.contains(rule)
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
pub fn require_accounted(audit: &Audit, tool: &super::types::Tool) -> Result<(), Error> {
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

/// Refuse absolute, empty and traversing paths before boundary access.
///
/// # Errors
/// Refuses empty, absolute, traversing or otherwise non-normal relative paths.
pub fn relative_path(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!(
            "unsafe corpus path {}",
            path.display()
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
    let has_rule_finding = |case: &super::types::Case| {
        match case.comparison {
        super::types::CaseComparison::Rule => !case.expected.is_empty(),
        super::types::CaseComparison::Full => case.expected.iter().any(|finding| {
            matches!(&finding.identity, super::types::ExpectedRule::Explicit(found) if found == rule)
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
        if (case.files.is_empty() && case.requested_files.as_ref().is_none_or(Vec::is_empty))
            || case.name.0.trim().is_empty()
            || !names.insert(&case.name)
        {
            return Err(Error::Invalid(
                super::constants::INVALID_CASE_IDENTITY.into(),
            ));
        }
        for name in case.files.keys() {
            relative_path(name.0.as_ref())?;
        }
        if let Some(requested) = &case.requested_files {
            let selected: std::collections::BTreeSet<_> = requested.iter().collect();
            if requested.is_empty() || selected.len() != requested.len() {
                return Err(Error::Invalid(
                    "explicit source inventory must name each selected source exactly once".into(),
                ));
            }
            if let Some(context) = &case.context {
                if case.files.keys().any(|path| !selected.contains(path)) {
                    return Err(Error::Invalid(
                        "sealed disk context must select every staged readable source".into(),
                    ));
                }
                for path in requested {
                    relative_path(path.0.as_ref())?;
                    if !case.files.contains_key(path)
                        && !context_missing(&context.logical_root, &context.missing, path)
                    {
                        return Err(Error::Invalid(
                            "requested unavailable source lacks a context absence seal".into(),
                        ));
                    }
                }
            } else {
                if requested.iter().any(|path| !case.files.contains_key(path)) {
                    return Err(Error::Invalid(
                        "Given source selection lacks a staged file".into(),
                    ));
                }
                for path in case.files.keys().filter(|path| !selected.contains(path)) {
                    let inventory = case.given_project_inventory.as_ref().ok_or_else(|| {
                        Error::Invalid(
                            "unselected Given source requires an explicit project inventory".into(),
                        )
                    })?;
                    if !inventory.entries.iter().any(|entry| {
                        entry.path == *path && entry.state == ryo::linter::ProjectEntryState::File
                    }) {
                        return Err(Error::Invalid(
                            "unselected Given source lacks a matching file observation".into(),
                        ));
                    }
                }
            }
        }
        if let Some(values) = &case.file_metadata {
            for name in values.keys() {
                relative_path(name.0.as_ref())?;
                if !case.files.contains_key(name) {
                    return Err(Error::Invalid(
                        "file metadata names an undeclared source".into(),
                    ));
                }
            }
        }
        if case.reference_discrepancy.is_some()
            && case.comparison != super::types::CaseComparison::Full
        {
            return Err(Error::Invalid(
                "reference discrepancies require full comparison".into(),
            ));
        }
        for expected in case.expected.iter().chain(
            case.reference_discrepancy
                .iter()
                .flat_map(|item| &item.expected),
        ) {
            if case.comparison == super::types::CaseComparison::Full
                && matches!(expected.identity, super::types::ExpectedRule::Parent)
            {
                return Err(Error::Invalid(
                    super::constants::MISSING_DIAGNOSTIC_IDENTITY.into(),
                ));
            }
            let source = match case.files.get(&expected.file) {
                Some(source) => source,
                None if case
                    .requested_files
                    .as_ref()
                    .is_some_and(|paths| paths.contains(&expected.file))
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
    manifest: &super::types::ContextManifest,
    file: &ryo::linter::RelativePath,
) -> bool {
    context_missing(&manifest.logical_root, &manifest.missing, file)
}

/// Match logical input paths using the same harmless `.` normalization as context materialization.
fn context_missing(
    root: &Path,
    missing: &[std::path::PathBuf],
    file: &ryo::linter::RelativePath,
) -> bool {
    let input = context_input_path(root, file);
    missing.iter().any(|path| {
        path.components()
            .filter(|part| *part != Component::CurDir)
            .eq(input.components().filter(|part| *part != Component::CurDir))
    })
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
