//! The cases a staged profile authors, as the registered case documents the inventory verification checks: each case's
//! identity is the digest of its authored table, and a documented difference is validated against the staged evidence.
use super::super::canonical::schemas::document;
use super::super::canonical::schemas::{Canonical, invalid, table, text};
use super::super::case_identity::schemas::{file_metadata_digest, source_digest};
use super::super::case_wire::schemas::CaseWire;
use super::super::constants;
use super::super::context_identity::schemas::context_identity;
use super::super::difference::schemas::reference_discrepancy;
use super::super::findings::schemas::{finding, sorted_findings};
use super::super::project_inputs::schemas::project_inputs;
use super::super::types::CaseText;
use crate::outbound::inventory::types::CaseDocuments;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The registered document of one authored case.
fn case_document(
    stage: &Path,
    profile: &BTreeMap<String, Canonical>,
    mode: &str,
    owner: &str,
    case: &BTreeMap<String, Canonical>,
) -> Result<CaseWire, Error> {
    let name = case
        .get(constants::NAME_KEY)
        .cloned()
        .ok_or_else(|| invalid(constants::CASE_NAME))?;
    let sources = case
        .get(constants::FILES)
        .and_then(Canonical::as_table)
        .ok_or_else(|| invalid(constants::CASE_FILES))?;
    let expected = case
        .get(constants::EXPECTED_KEY)
        .and_then(Canonical::as_list)
        .unwrap_or_default()
        .iter()
        .map(|row| finding(row, owner))
        .collect::<Result<Vec<_>, _>>()?;
    let declaration = profile.get(constants::RULE_CONTEXT);
    let configuration = case.get(constants::RULE_CONTEXT);
    if configuration.is_some() && declaration.is_none() {
        return Err(invalid(constants::RULE_CONTEXT_OVERRIDE));
    }
    let prepared = match declaration {
        Some(declaration) => {
            let declaration = declaration
                .as_table()
                .filter(|declaration| {
                    declaration.len() == 2
                        && declaration.contains_key(constants::TABLES)
                        && declaration.contains_key(constants::DEFAULTS_KEY)
                })
                .ok_or_else(|| invalid(constants::RULE_CONTEXT_DECLARATION))?;
            Some(table([
                (
                    constants::TABLES,
                    declaration
                        .get(constants::TABLES)
                        .cloned()
                        .unwrap_or(Canonical::Null),
                ),
                (
                    constants::CONFIGURATION,
                    configuration
                        .or_else(|| declaration.get(constants::DEFAULTS_KEY))
                        .cloned()
                        .unwrap_or(Canonical::Null),
                ),
            ]))
        }
        None => None,
    };
    let selected = case.get(constants::SELECTED_RULES).cloned().or_else(|| {
        prepared.as_ref().map(|_| {
            profile
                .get(constants::DEFAULTS_KEY)
                .cloned()
                .unwrap_or(Canonical::Null)
        })
    });
    let metadata = case.get(constants::FILE_METADATA).cloned();
    let metadata_digest = match &metadata {
        Some(values) => Some(file_metadata_digest(
            values
                .as_table()
                .ok_or_else(|| invalid(constants::FILE_METADATA_FLAGS))?,
        )?),
        None => None,
    };
    let inputs = project_inputs(case, profile)?;
    let inputs_digest = inputs
        .as_ref()
        .map(|inputs| {
            serde_json::to_string(inputs)
                .map(|text| format!("{:x}", Sha256::digest(text.as_bytes())))
                .map_err(|error| Error::Json(Box::new(error)))
        })
        .transpose()?;
    let files = match case.get(constants::REQUESTED_FILES) {
        Some(requested) => {
            let mut files = requested
                .as_list()
                .ok_or_else(|| invalid(constants::SELECTED_FILES))?
                .to_vec();
            files.sort_by(|left, right| left.as_text().cmp(&right.as_text()));
            files
        }
        None => sources.keys().map(|file| text(file)).collect(),
    };
    Ok(CaseWire {
        rule: owner.to_owned(),
        name,
        case_sha256: Canonical::Table(case.clone()).digest()?,
        source_sha256: source_digest(sources)?,
        files,
        comparison: case
            .get(constants::COMPARISON)
            .cloned()
            .unwrap_or_else(|| text(constants::RULE)),
        expected: sorted_findings(expected),
        project_context_mode: mode.to_owned(),
        context_identity: context_identity(case, mode)?,
        reference_discrepancy: reference_discrepancy(stage, profile, case)?,
        expectation_contract: None,
        rule_context: prepared,
        rule_observations: case.get(constants::RULE_OBSERVATIONS_FIELD).cloned(),
        selected_rules: selected,
        file_observation: profile.get(constants::FILE_OBSERVATION).cloned(),
        file_metadata: metadata,
        file_metadata_sha256: metadata_digest,
        inputs,
        project_inputs_sha256: inputs_digest,
    })
}

/// Every case the staged profile at `stage` authors, as registered case documents in rule and name order.
pub(in super::super) fn case_inventory(stage: &Path) -> Result<CaseDocuments, Error> {
    let profile_path =
        stage.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
    let profile = if profile_path.is_file() {
        document(&profile_path)?
    } else {
        BTreeMap::new()
    };
    let mode = profile
        .get(constants::PROJECT_CONTEXT_MODE)
        .and_then(Canonical::as_text)
        .unwrap_or(constants::ISOLATED)
        .to_owned();
    if mode != constants::ISOLATED && mode != constants::LISTED_DISK {
        return Err(invalid(constants::CONTEXT_MODE));
    }
    let mut owners = std::fs::read_dir(stage.join(constants::RULES_DIRECTORY))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    owners.sort();
    let mut documents = Vec::new();
    for owner in owners {
        let cases = owner.join(ryo_benchmark_contracts::domain::requests::constants::CASES_FILE);
        if !cases.is_file() {
            continue;
        }
        let name = owner
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid(constants::CASE_NAME))?
            .to_owned();
        let authored = document(&cases)?;
        for case in authored
            .get(constants::CASES)
            .and_then(Canonical::as_list)
            .unwrap_or_default()
        {
            let case = case
                .as_table()
                .ok_or_else(|| invalid(constants::CASE_NAME))?;
            documents.push(case_document(stage, &profile, &mode, &name, case)?);
        }
    }
    let identity = |document: &CaseWire| {
        (
            document.rule.clone(),
            document.name.as_text().unwrap_or_default().to_owned(),
        )
    };
    documents.sort_by_key(identity);
    if documents.is_empty()
        || documents
            .iter()
            .map(identity)
            .collect::<BTreeSet<_>>()
            .len()
            != documents.len()
    {
        return Err(invalid(constants::CASE_IDENTITIES));
    }
    documents
        .into_iter()
        .map(|document| {
            let text =
                serde_json::to_string(&document).map_err(|error| Error::Json(Box::new(error)))?;
            CaseDocument::try_from(CaseText(&text))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(CaseDocuments)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
