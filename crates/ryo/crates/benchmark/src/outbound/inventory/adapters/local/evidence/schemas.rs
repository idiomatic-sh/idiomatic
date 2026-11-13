//! The evidence rows of a documented difference: each matches its authority and its staged bytes.
use super::super::canonical::schemas::{Canonical, invalid};
use super::super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

/// The SHA-256 of a file's bytes.
pub(in super::super) fn file_digest(path: &Path) -> Result<String, Error> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

/// A lowercase SHA-256 digest.
pub(in super::super) fn lower_digest(value: &str) -> bool {
    value.len() == constants::DIGEST_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Every evidence row of a documented difference matches its authority and its staged bytes.
pub(in super::super) fn evidence(
    stage: &Path,
    profile: &BTreeMap<String, Canonical>,
    basis: &str,
    rows: Option<&Canonical>,
) -> Result<(), Error> {
    let rows = rows
        .and_then(Canonical::as_list)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| invalid(constants::EVIDENCE_REQUIRED))?;
    let mut kinds = Vec::new();
    for row in rows {
        let row = row
            .as_table()
            .ok_or_else(|| invalid(constants::EVIDENCE_INCOMPLETE))?;
        let keys: BTreeSet<_> = row.keys().map(String::as_str).collect();
        let field = |key: &str| {
            row.get(key)
                .and_then(Canonical::as_text)
                .unwrap_or_default()
        };
        let kind = field(constants::KIND);
        kinds.push(kind.to_owned());
        let expected: BTreeSet<_> = match (basis, kind) {
            (constants::LANGUAGE_DEFECT, constants::LANGUAGE_SPEC | constants::INDEPENDENT_RUN) => {
                constants::LANGUAGE_FIELDS.into_iter().collect()
            }
            (constants::LANGUAGE_DEFECT, _) => return Err(invalid(constants::EVIDENCE_CONTRACT)),
            (_, constants::PUBLIC_CONTRACT) => constants::CONTRACT_FIELDS.into_iter().collect(),
            (_, constants::PRIOR_WORKER) => constants::PRIOR_FIELDS.into_iter().collect(),
            (_, _) => return Err(invalid(constants::EVIDENCE_KIND)),
        };
        if keys != expected {
            return Err(invalid(constants::EVIDENCE_INCOMPLETE));
        }
        let valid = match kind {
            constants::LANGUAGE_SPEC | constants::INDEPENDENT_RUN => {
                !field(constants::LANGUAGE_VERSION).trim().is_empty()
                    && !field(constants::VERSION_RATIONALE).trim().is_empty()
                    && row.get(constants::TARGET_CONFIGURATION)
                        == profile.get(constants::PARSER_CONFIGURATION)
            }
            constants::PUBLIC_CONTRACT => {
                field(constants::SOURCE_REVISION).len() == constants::REVISION_HEX_LENGTH
                    && field(constants::SOURCE_REVISION)
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    && !field(constants::SOURCE_PATH).trim().is_empty()
                    && !field(constants::RATIONALE).trim().is_empty()
            }
            _ => {
                constants::PRIOR_DIGESTS
                    .iter()
                    .all(|key| lower_digest(field(key)))
                    && !field(constants::WITNESS_CASE).trim().is_empty()
                    && !field(constants::RATIONALE).trim().is_empty()
            }
        };
        if !valid {
            return Err(invalid(constants::EVIDENCE_CONTRACT));
        }
        let path = field(constants::PATH);
        if !Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
            || path.is_empty()
        {
            return Err(invalid(constants::UNSAFE_EVIDENCE));
        }
        let root = stage.canonicalize()?;
        let resolved = root
            .join(path)
            .canonicalize()
            .map_err(|_| invalid(constants::STALE_EVIDENCE))?;
        if !resolved.starts_with(&root)
            || !resolved.is_file()
            || file_digest(&resolved)? != field(constants::SHA256_KEY)
        {
            return Err(invalid(constants::STALE_EVIDENCE));
        }
    }
    kinds.sort();
    if basis == constants::TOOL_CONTRACT
        && kinds != [constants::PRIOR_WORKER, constants::PUBLIC_CONTRACT]
    {
        return Err(invalid(constants::CONTRACT_EVIDENCE_PAIR));
    }
    Ok(())
}
