//! Pure validation shared by artifact auditing and case execution.
use super::types::{Cases, audit::value::Audit};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use std::path::{Component, Path};

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
        if case.files.is_empty() || case.name.0.trim().is_empty() || !names.insert(&case.name) {
            return Err(Error::Invalid(
                super::constants::INVALID_CASE_IDENTITY.into(),
            ));
        }
        for name in case.files.keys() {
            relative_path(name.0.as_ref())?;
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
            let source = case
                .files
                .get(&expected.file)
                .ok_or_else(|| Error::Invalid(super::constants::MISSING_SOURCE.into()))?;
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
