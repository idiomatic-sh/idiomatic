//! The listed context a case was authored against, framed as the benchmark seals it.
use super::super::canonical::schemas::{Canonical, invalid, table, text};
use super::super::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

/// The listed context a case was authored against, framed as the benchmark seals it; isolated cases have none.
pub(in super::super) fn context_identity(
    case: &BTreeMap<String, Canonical>,
    mode: &str,
) -> Result<Canonical, Error> {
    let context = case.get(constants::CONTEXT);
    if mode == constants::ISOLATED {
        return if context.is_none() {
            Ok(Canonical::Null)
        } else {
            Err(invalid(constants::ISOLATED_CONTEXT))
        };
    }
    let context = context
        .and_then(Canonical::as_table)
        .filter(|context| {
            context.keys().all(|key| {
                [
                    constants::LOGICAL_ROOT,
                    constants::AUXILIARY_FILES,
                    constants::MISSING,
                ]
                .contains(&key.as_str())
            })
        })
        .ok_or_else(|| invalid(constants::LISTED_CONTEXT))?;
    let logical = context
        .get(constants::LOGICAL_ROOT)
        .and_then(Canonical::as_text)
        .filter(|root| {
            !root.is_empty()
                && !Path::new(root).is_absolute()
                && !Path::new(root)
                    .components()
                    .any(|part| part == Component::ParentDir)
        })
        .ok_or_else(|| invalid(constants::UNSAFE_CONTEXT))?;
    let relative = |name: &str| -> Result<String, Error> {
        let path = Path::new(name);
        if name.is_empty()
            || path.is_absolute()
            || path.components().any(|part| part == Component::ParentDir)
        {
            return Err(invalid(constants::UNSAFE_CONTEXT));
        }
        Ok(posix(name))
    };
    let digests = |files: Option<&Canonical>,
                   root: Option<&str>|
     -> Result<BTreeMap<String, String>, Error> {
        let mut found = BTreeMap::new();
        for (name, value) in files.and_then(Canonical::as_table).into_iter().flatten() {
            let named = relative(name)?;
            let named = match root {
                Some(root) => posix(&format!("{root}/{named}")),
                None => named,
            };
            let source = value
                .as_text()
                .ok_or_else(|| invalid(constants::CASE_FILES))?;
            let _previous = found.insert(named, format!("{:x}", Sha256::digest(source.as_bytes())));
        }
        Ok(found)
    };
    let sources = digests(case.get(constants::FILES), Some(logical))?;
    let auxiliary = digests(context.get(constants::AUXILIARY_FILES), None)?;
    let mut missing = context
        .get(constants::MISSING)
        .and_then(Canonical::as_list)
        .unwrap_or_default()
        .iter()
        .map(|name| {
            relative(
                name.as_text()
                    .ok_or_else(|| invalid(constants::UNSAFE_CONTEXT))?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    missing.sort();
    let present: BTreeSet<_> = sources.keys().chain(auxiliary.keys()).cloned().collect();
    if sources.keys().any(|name| auxiliary.contains_key(name))
        || missing.iter().any(|name| present.contains(name))
        || missing.iter().collect::<BTreeSet<_>>().len() != missing.len()
    {
        return Err(invalid(constants::CONTEXT_ROLES_OVERLAP));
    }
    let path_digests = |files: &BTreeMap<String, String>| {
        files
            .iter()
            .map(|(name, digest)| {
                (
                    std::path::PathBuf::from(name),
                    ArtifactDigest(digest.clone()),
                )
            })
            .collect()
    };
    let seal = ryo_benchmark_contracts::domain::context::service::fingerprint(&ContextManifest {
        boundary: std::path::PathBuf::new(),
        logical_root: std::path::PathBuf::from(logical),
        auxiliary_files: path_digests(&auxiliary),
        source_files: path_digests(&sources),
        missing: missing.iter().map(std::path::PathBuf::from).collect(),
    });
    Ok(table([
        (constants::MODE, text(mode)),
        (constants::LOGICAL_ROOT_KEY, text(logical)),
        (constants::MANIFEST_HASH, text(&seal.0)),
        (
            constants::PRESENT_PATHS,
            Canonical::List(present.iter().map(|name| text(name)).collect()),
        ),
        (
            constants::MISSING_PATHS,
            Canonical::List(missing.iter().map(|name| text(name)).collect()),
        ),
    ]))
}

/// A relative path as the authoring spells it: its normal components joined by `/`, `.` when there are none.
fn posix(path: &str) -> String {
    let parts = Path::new(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        constants::CURRENT_DIRECTORY.to_owned()
    } else {
        parts.join("/")
    }
}
