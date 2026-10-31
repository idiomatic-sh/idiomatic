//! Pure validation shared by artifact auditing and case execution.
use super::types::{Cases, audit::value::Audit};
use crate::domain::diagnostics::errors::Error;
use std::path::{Component, Path};

/// Require a nonempty complete implementation inventory.
pub fn require_complete(audit: &Audit) -> Result<(), Error> {
    if audit.implemented.is_empty() && audit.missing.is_empty() {
        return Err(Error::Invalid("empty audit is not default coverage".into()));
    }
    if audit.missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "default coverage incomplete: {} missing rules: {}",
            audit.missing.len(),
            audit.missing.join(", ")
        )))
    }
}

/// Every default is either authored or explicitly deferred; no gap becomes a match.
pub fn require_accounted(audit: &Audit, tool: &super::types::Tool) -> Result<(), Error> {
    if audit.implemented.is_empty() {
        return Err(Error::Invalid("no authored default subset".into()));
    }
    for (rule, issue) in &tool.deferred {
        if !audit.missing.contains(rule) || issue.is_empty() {
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
pub fn relative_path(path: &str) -> Result<(), Error> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!("unsafe corpus path {path:?}")));
    }
    Ok(())
}

/// Require independent positive/negative cases with valid source spans and names.
pub fn validate_cases(cases: &Cases) -> Result<(), Error> {
    if !cases.cases.iter().any(|case| case.expected.is_empty())
        || !cases.cases.iter().any(|case| !case.expected.is_empty())
    {
        return Err(Error::Invalid(
            "a rule requires positive and negative cases".into(),
        ));
    }
    validate_case_sources(cases)
}

/// Validate case identity, source coverage and byte ranges without asserting a positive finding.
pub fn validate_case_sources(cases: &Cases) -> Result<(), Error> {
    if cases.cases.is_empty() {
        return Err(Error::Invalid("case evidence must be nonempty".into()));
    }
    let mut names = std::collections::BTreeSet::new();
    for case in &cases.cases {
        if case.files.is_empty() || case.name.trim().is_empty() || !names.insert(&case.name) {
            return Err(Error::Invalid(
                "case names must be unique and sources nonempty".into(),
            ));
        }
        for name in case.files.keys() {
            relative_path(name)?;
        }
        for expected in &case.expected {
            let source = case
                .files
                .get(&expected.file)
                .ok_or_else(|| Error::Invalid("expected finding outside case".into()))?;
            if expected.start > expected.end
                || !source.is_char_boundary(expected.start)
                || !source.is_char_boundary(expected.end)
            {
                return Err(Error::Invalid(
                    "expected finding has invalid UTF-8 span".into(),
                ));
            }
        }
    }
    Ok(())
}
