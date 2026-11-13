// ignore: idiomatic.arch/file-length - one authored-case document read top to bottom: the case identity, its context identity, its project inputs and the documented difference it carries share the raw case table, and no placed role splits them
//! The cases a staged profile authors, as the registered case documents the inventory verification checks: each case's
//! identity is the digest of its authored table, and a documented difference is validated against the staged evidence.
use super::super::case::schemas::{Canonical, CaseWire, InputsWire};
use super::super::constants;
use super::super::types::CaseText;
use crate::outbound::inventory::types::CaseDocuments;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

/// A refusal of the authored cases.
fn invalid(text: &str) -> Error {
    Error::Invalid(text.to_owned())
}

/// A table of texts, as the authoring builds them.
fn table<const N: usize>(entries: [(&str, Canonical); N]) -> Canonical {
    Canonical::Table(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// A text value.
fn text(value: &str) -> Canonical {
    Canonical::Text(value.to_owned())
}

/// Read one TOML document as a table of canonical values.
pub(in super::super) fn document(path: &Path) -> Result<BTreeMap<String, Canonical>, Error> {
    let parsed: toml::Table = toml::from_str(&std::fs::read_to_string(path)?)?;
    match Canonical::try_from(toml::Value::Table(parsed))? {
        Canonical::Table(table) => Ok(table),
        Canonical::Null
        | Canonical::Flag(_)
        | Canonical::Integer(_)
        | Canonical::Float(_)
        | Canonical::Text(_)
        | Canonical::List(_) => Err(invalid(constants::UNSUPPORTED_DOCUMENT_VALUE)),
    }
}

/// The SHA-256 of a file's bytes.
pub(in super::super) fn file_digest(path: &Path) -> Result<String, Error> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

/// A relative path with no empty, current, parent or root component.
fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// A lowercase SHA-256 digest.
fn lower_digest(value: &str) -> bool {
    value.len() == constants::DIGEST_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The framed SHA-256 of a case's declared file metadata, as the benchmark contracts frame it.
fn file_metadata_digest(values: &BTreeMap<String, Canonical>) -> Result<String, Error> {
    let entries = values
        .iter()
        .map(|(path, executable)| match executable {
            Canonical::Flag(executable) => {
                Ok((ryo::linter::RelativePath(path.as_str().into()), *executable))
            }
            Canonical::Null
            | Canonical::Integer(_)
            | Canonical::Float(_)
            | Canonical::Text(_)
            | Canonical::List(_)
            | Canonical::Table(_) => Err(invalid(constants::FILE_METADATA_FLAGS)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    ArtifactDigest::file_metadata(Some(entries.iter().map(|(path, executable)| {
        FileMetadataEntry {
            path,
            executable: if *executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        }
    })))
    .map_err(|error| Error::Invalid(error.to_string()))?
    .map(|digest| digest.0)
    .ok_or_else(|| invalid(constants::FILE_METADATA_FLAGS))
}

/// The SHA-256 of a case's sources: each name, a zero byte and its text, in name order.
fn source_digest(files: &BTreeMap<String, Canonical>) -> Result<String, Error> {
    let mut digest = Sha256::new();
    for (file, source) in files {
        digest.update(file.as_bytes());
        digest.update([0]);
        digest.update(
            source
                .as_text()
                .ok_or_else(|| invalid(constants::CASE_FILES))?
                .as_bytes(),
        );
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// The listed context a case was authored against, framed as the benchmark seals it; isolated cases have none.
fn context_identity(case: &BTreeMap<String, Canonical>, mode: &str) -> Result<Canonical, Error> {
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

/// The explicit project inputs of a case with the profile's search roots, when either is declared.
fn project_inputs(
    case: &BTreeMap<String, Canonical>,
    profile: &BTreeMap<String, Canonical>,
) -> Result<Option<InputsWire>, Error> {
    let roots = case
        .get(constants::PROJECT_SEARCH_ROOTS)
        .or_else(|| profile.get(constants::PROJECT_SEARCH_ROOTS))
        .cloned();
    let default_inventory = table([
        (constants::COMPLETENESS, text(constants::PARTIAL)),
        (constants::ENTRIES, Canonical::List(Vec::new())),
    ]);
    let inventory = case
        .get(constants::GIVEN_PROJECT_INVENTORY)
        .cloned()
        .unwrap_or_else(|| default_inventory.clone());
    if let Some(roots) = &roots {
        let valid = roots.as_list().is_some_and(|roots| {
            roots.iter().all(|root| {
                root.as_text().is_some_and(|root| {
                    root.is_empty()
                        || (!Path::new(root).is_absolute()
                            && Path::new(root)
                                .components()
                                .all(|part| matches!(part, Component::Normal(_))))
                })
            })
        });
        if !valid {
            return Err(invalid(constants::SEARCH_ROOTS));
        }
    }
    let shape = inventory.as_table().filter(|fields| {
        fields.len() == 2
            && fields
                .get(constants::COMPLETENESS)
                .and_then(Canonical::as_text)
                .is_some_and(|completeness| {
                    [constants::PARTIAL, constants::COMPLETE_SCOPE].contains(&completeness)
                })
            && fields
                .get(constants::ENTRIES)
                .and_then(Canonical::as_list)
                .is_some()
    });
    let Some(fields) = shape else {
        return Err(invalid(constants::GIVEN_INVENTORY_SHAPE));
    };
    for entry in fields
        .get(constants::ENTRIES)
        .and_then(Canonical::as_list)
        .unwrap_or_default()
    {
        let valid = entry.as_table().is_some_and(|entry| {
            entry.len() == 2
                && entry
                    .get(constants::PATH)
                    .and_then(Canonical::as_text)
                    .is_some_and(safe_relative)
                && entry
                    .get(constants::STATE)
                    .and_then(Canonical::as_text)
                    .is_some_and(|state| constants::ENTRY_STATES.contains(&state))
        });
        if !valid {
            return Err(invalid(constants::GIVEN_ENTRY));
        }
    }
    let complete = fields
        .get(constants::COMPLETENESS)
        .and_then(Canonical::as_text)
        == Some(constants::COMPLETE_SCOPE);
    match roots {
        None if complete => Err(invalid(constants::COMPLETE_NEEDS_ROOTS)),
        None if inventory == default_inventory => Ok(None),
        roots => Ok(Some(InputsWire {
            given_project_inventory: inventory,
            project_search_roots: roots.unwrap_or(Canonical::Null),
        })),
    }
}

/// One expected finding as the authoring normalizes it, with its rule defaulting to the case's owner.
fn finding(row: &Canonical, owner: &str) -> Result<Canonical, Error> {
    let row = row
        .as_table()
        .ok_or_else(|| invalid(constants::EXPECTED_ROW))?;
    let field = |key: &str| {
        row.get(key)
            .cloned()
            .ok_or_else(|| invalid(constants::EXPECTED_ROW))
    };
    Ok(table([
        (
            constants::RULE,
            row.get(constants::RULE)
                .cloned()
                .unwrap_or_else(|| text(owner)),
        ),
        (constants::FILE, field(constants::FILE)?),
        (constants::START, field(constants::START)?),
        (constants::END, field(constants::END)?),
    ]))
}

/// The order expected findings are written in: rule, file, start, end.
fn finding_key(row: &Canonical) -> (String, String, i64, i64) {
    let field = |key| row.as_table().and_then(|row| row.get(key));
    (
        field(constants::RULE)
            .and_then(Canonical::as_text)
            .unwrap_or_default()
            .to_owned(),
        field(constants::FILE)
            .and_then(Canonical::as_text)
            .unwrap_or_default()
            .to_owned(),
        field(constants::START)
            .and_then(Canonical::as_integer)
            .unwrap_or_default(),
        field(constants::END)
            .and_then(Canonical::as_integer)
            .unwrap_or_default(),
    )
}

/// Findings sorted as the authoring writes them.
fn sorted_findings(mut rows: Vec<Canonical>) -> Canonical {
    rows.sort_by_key(finding_key);
    Canonical::List(rows)
}

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
fn reference_discrepancy(
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

/// Every evidence row of a documented difference matches its authority and its staged bytes.
fn evidence(
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
