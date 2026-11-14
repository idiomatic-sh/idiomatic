//! A case's documented difference from the reference, validated against the staged profile and its evidence, in its
//! registered form.
use super::super::canonical::schemas::{Canonical, invalid, table, text};
use super::super::case_identity::schemas::{file_metadata_digest, source_digest};
use super::super::constants;
use super::super::evidence::schemas::{evidence, file_digest, lower_digest};
use super::super::findings::schemas::sorted_findings;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The files a case lints, excluding staged context sources, sorted.
fn selected_files(case: &BTreeMap<String, Canonical>) -> Result<Vec<String>, Error> {
    let supplied = case
        .get(constants::FILES)
        .and_then(Canonical::as_table)
        .ok_or_else(|| invalid(constants::SELECTED_FILES))?;
    let mut selected = match case.get(constants::REQUESTED_FILES) {
        Some(requested) => requested
            .as_list()
            .ok_or_else(|| invalid(constants::SELECTED_FILES))?
            .iter()
            .map(|file| {
                file.as_text()
                    .filter(|file| supplied.contains_key(*file))
                    .map(str::to_owned)
                    .ok_or_else(|| invalid(constants::SELECTED_FILES))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => supplied.keys().cloned().collect(),
    };
    if selected.iter().collect::<BTreeSet<_>>().len() != selected.len() {
        return Err(invalid(constants::SELECTED_FILES));
    }
    selected.sort();
    Ok(selected)
}

/// The rows of a full discrepancy expectation: explicit identities inside the selected sources, on character boundaries.
fn expectation_rows(
    rows: &Canonical,
    case: &BTreeMap<String, Canonical>,
    files: &[String],
) -> Result<Vec<Canonical>, Error> {
    let sources = case
        .get(constants::FILES)
        .and_then(Canonical::as_table)
        .ok_or_else(|| invalid(constants::CASE_FILES))?;
    let mut normalized = Vec::new();
    for row in rows
        .as_list()
        .ok_or_else(|| invalid(constants::DISCREPANCY_ROWS))?
    {
        let fields = row.as_table().filter(|fields| {
            fields.len() == 4
                && [
                    constants::RULE,
                    constants::FILE,
                    constants::START,
                    constants::END,
                ]
                .iter()
                .all(|key| fields.contains_key(*key))
        });
        let Some(fields) = fields else {
            return Err(invalid(constants::DISCREPANCY_ROWS));
        };
        let file = fields
            .get(constants::FILE)
            .and_then(Canonical::as_text)
            .filter(|file| files.iter().any(|selected| selected == file));
        let rule = fields
            .get(constants::RULE)
            .and_then(Canonical::as_text)
            .filter(|rule| !rule.is_empty());
        let (Some(file), Some(_rule)) = (file, rule) else {
            return Err(invalid(constants::DISCREPANCY_OUTSIDE));
        };
        let source = sources
            .get(file)
            .and_then(Canonical::as_text)
            .unwrap_or_default();
        let bounds = (
            fields.get(constants::START).and_then(Canonical::as_integer),
            fields.get(constants::END).and_then(Canonical::as_integer),
        );
        let (Some(start), Some(end)) = bounds else {
            return Err(invalid(constants::DISCREPANCY_RANGE));
        };
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return Err(invalid(constants::DISCREPANCY_RANGE));
        };
        if start > end
            || end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            return Err(invalid(constants::DISCREPANCY_RANGE));
        }
        normalized.push(row.clone());
    }
    Ok(normalized)
}

/// The output a discrepancy binds: the selected files, every finding in order, and no skip.
fn discrepancy_output(files: &[String], rows: Vec<Canonical>) -> Canonical {
    table([
        (
            constants::FILES_KEY,
            Canonical::List(files.iter().map(|file| text(file)).collect()),
        ),
        (constants::DIAGNOSTICS_KEY, sorted_findings(rows)),
        (constants::SKIPPED_KEY, Canonical::List(Vec::new())),
    ])
}

/// Validate a case's documented difference against the staged profile and evidence, and answer its registered form.
pub(in super::super) fn reference_discrepancy(
    stage: &Path,
    profile: &BTreeMap<String, Canonical>,
    case: &BTreeMap<String, Canonical>,
) -> Result<Canonical, Error> {
    let language = case.get(constants::REFERENCE_DISCREPANCY);
    let contract = case.get(constants::TOOL_CONTRACT_DIFFERENCE);
    let (basis, annotation) = match (language, contract) {
        (Some(_), Some(_)) => return Err(invalid(constants::TWO_AUTHORITIES)),
        (None, None) => return Ok(Canonical::Null),
        (Some(annotation), None) => (constants::LANGUAGE_DEFECT, annotation),
        (None, Some(annotation)) => (constants::TOOL_CONTRACT, annotation),
    };
    let fields = annotation
        .as_table()
        .ok_or_else(|| invalid(constants::DISCREPANCY_SCHEMA))?;
    let approved = profile
        .get(constants::SHA256_KEY)
        .cloned()
        .unwrap_or(Canonical::Null);
    if basis == constants::LANGUAGE_DEFECT && fields.contains_key(constants::PROCESS) {
        return process_discrepancy(case, fields, approved);
    }
    let keys: BTreeSet<_> = fields.keys().map(String::as_str).collect();
    if keys
        != BTreeSet::from([
            constants::EXPECTED_KEY,
            constants::IDENTITY,
            constants::EVIDENCE,
            constants::REASON,
        ])
    {
        return Err(invalid(constants::DISCREPANCY_SCHEMA));
    }
    let reason = fields
        .get(constants::REASON)
        .and_then(Canonical::as_text)
        .unwrap_or_default();
    if case
        .get(constants::COMPARISON)
        .and_then(Canonical::as_text)
        .unwrap_or(constants::RULE)
        != constants::FULL
        || reason.trim().is_empty()
    {
        return Err(invalid(constants::DISCREPANCY_FULL));
    }
    let identity = fields
        .get(constants::IDENTITY)
        .and_then(Canonical::as_table)
        .ok_or_else(|| invalid(constants::DISCREPANCY_IDENTITY))?;
    let names: BTreeSet<_> = identity.keys().map(String::as_str).collect();
    let required: BTreeSet<_> = constants::IDENTITY_FIELDS.into_iter().collect();
    if !required.is_subset(&names)
        || names
            .iter()
            .any(|name| !required.contains(name) && *name != constants::FILE_METADATA_SHA256_KEY)
    {
        return Err(invalid(constants::DISCREPANCY_IDENTITY));
    }
    let metadata = match case.get(constants::FILE_METADATA) {
        Some(values) => Some(file_metadata_digest(
            values
                .as_table()
                .ok_or_else(|| invalid(constants::FILE_METADATA_FLAGS))?,
        )?),
        None => None,
    };
    if identity
        .get(constants::FILE_METADATA_SHA256_KEY)
        .and_then(Canonical::as_text)
        != metadata.as_deref()
    {
        return Err(invalid(constants::STALE_METADATA));
    }
    let digest = |key: &str| {
        identity
            .get(key)
            .and_then(Canonical::as_text)
            .unwrap_or_default()
    };
    if names
        .iter()
        .filter(|name| **name != constants::TOOL && **name != constants::VERSION)
        .any(|name| !lower_digest(digest(name)))
    {
        return Err(invalid(constants::DISCREPANCY_DIGESTS));
    }
    let staged = stage.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
    let pinned = approved.as_list().unwrap_or_default();
    if identity.get(constants::TOOL) != profile.get(constants::TOOL)
        || identity.get(constants::VERSION) != profile.get(constants::VERSION)
        || !pinned
            .iter()
            .any(|pin| pin.as_text() == Some(digest(constants::OBSERVED_EXECUTABLE)))
        || digest(constants::PROFILE_SHA256) != file_digest(&staged)?
    {
        return Err(invalid(constants::STALE_TOOL_PROFILE));
    }
    let sources = case
        .get(constants::FILES)
        .and_then(Canonical::as_table)
        .ok_or_else(|| invalid(constants::CASE_FILES))?;
    if source_digest(sources)? != digest(constants::SOURCE_SHA256) {
        return Err(invalid(constants::STALE_SOURCE));
    }
    let files = selected_files(case)?;
    let intended = discrepancy_output(
        &files,
        expectation_rows(
            case.get(constants::EXPECTED_KEY)
                .unwrap_or(&Canonical::Null),
            case,
            &files,
        )?,
    );
    let reference = discrepancy_output(
        &files,
        expectation_rows(
            fields
                .get(constants::EXPECTED_KEY)
                .unwrap_or(&Canonical::Null),
            case,
            &files,
        )?,
    );
    if intended == reference {
        return Err(invalid(constants::IDENTICAL_EXPECTATIONS));
    }
    if intended.digest()? != digest(constants::INTENDED_OUTPUT)
        || reference.digest()? != digest(constants::REFERENCE_OUTPUT)
    {
        return Err(invalid(constants::STALE_EXPECTATIONS));
    }
    evidence(stage, profile, basis, fields.get(constants::EVIDENCE))?;
    let diagnostics = |output: Canonical| {
        output
            .as_table()
            .and_then(|output| output.get(constants::DIAGNOSTICS_KEY))
            .cloned()
            .unwrap_or(Canonical::Null)
    };
    Ok(table([
        (constants::BASIS, text(basis)),
        (constants::IDENTITY, Canonical::Table(identity.clone())),
        (constants::APPROVED, approved),
        (constants::REFERENCE_EXPECTED, diagnostics(reference)),
        (constants::INTENDED_EXPECTED, diagnostics(intended)),
        (constants::REASON, text(reason)),
    ]))
}

/// A documented process failure, as registered: its identity, approved executables, intended findings and the failure.
fn process_discrepancy(
    case: &BTreeMap<String, Canonical>,
    fields: &BTreeMap<String, Canonical>,
    approved: Canonical,
) -> Result<Canonical, Error> {
    let keys: BTreeSet<_> = fields.keys().map(String::as_str).collect();
    if keys
        != BTreeSet::from([
            constants::PROCESS,
            constants::IDENTITY,
            constants::EVIDENCE,
            constants::REASON,
        ])
        || case.get(constants::COMPARISON).and_then(Canonical::as_text) != Some(constants::FULL)
    {
        return Err(invalid(constants::PROCESS_CONTRACT));
    }
    let process = fields
        .get(constants::PROCESS)
        .cloned()
        .unwrap_or(Canonical::Null);
    if process
        .as_table()
        .and_then(|process| process.get(constants::KIND))
        .and_then(Canonical::as_text)
        != Some(constants::PROCESS_FAILURE_KIND)
    {
        return Err(invalid(constants::PROCESS_VARIANT));
    }
    let mut intended = Vec::new();
    for row in case
        .get(constants::EXPECTED_KEY)
        .and_then(Canonical::as_list)
        .unwrap_or_default()
    {
        if row.as_table().is_none_or(|row| {
            row.len() != 4
                || ![
                    constants::RULE,
                    constants::FILE,
                    constants::START,
                    constants::END,
                ]
                .iter()
                .all(|key| row.contains_key(*key))
        }) {
            return Err(invalid(constants::PROCESS_FINDINGS));
        }
        intended.push(row.clone());
    }
    Ok(table([
        (
            constants::IDENTITY,
            fields
                .get(constants::IDENTITY)
                .cloned()
                .unwrap_or(Canonical::Null),
        ),
        (constants::APPROVED, approved),
        (constants::REFERENCE_EXPECTED, Canonical::Null),
        (constants::INTENDED_EXPECTED, sorted_findings(intended)),
        (
            constants::REASON,
            fields
                .get(constants::REASON)
                .cloned()
                .unwrap_or(Canonical::Null),
        ),
        (constants::PROCESS, process),
    ]))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
