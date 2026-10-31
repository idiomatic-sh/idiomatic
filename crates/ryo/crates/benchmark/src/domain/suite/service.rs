//! Pure validation shared by artifact auditing and case execution.
use super::types::{Audit, Cases};
use crate::domain::diagnostics::errors::Error;
use std::path::{Component, Path};

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

pub fn validate_cases(cases: &Cases) -> Result<(), Error> {
    if !cases.cases.iter().any(|case| case.expected.is_empty())
        || !cases.cases.iter().any(|case| !case.expected.is_empty())
    {
        return Err(Error::Invalid(
            "a rule requires positive and negative cases".into(),
        ));
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
