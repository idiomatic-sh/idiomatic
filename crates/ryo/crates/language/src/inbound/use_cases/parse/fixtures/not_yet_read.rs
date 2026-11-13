//! The relations a language has not read yet: the conformance lists each by name on every run, so that a stage of a language being
//! written is never a silence.

use std::io::Write as _;
use std::path::Path;

use super::super::service::absence as parse_absence;
use super::super::types::ParseEngine;
use super::findings::Findings;
use crate::inbound::use_cases::project::types::ProjectEngine;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::capabilities::types::OptionalCapability;
use ryo_relations::domain::relations::service::{all, folder_of, provenance_of};
use ryo_relations::domain::relations::types::{Provenance, Relation};

/// Where the command line's manifest is from a language's directory: its default features are the languages that ship.
const COMMAND_LINE_MANIFEST: &str = "../../cli/Cargo.toml";

/// The table of a manifest that names its features.
const FEATURES: &str = "features";

/// The feature of that table that lists the default features.
const DEFAULT_FEATURES: &str = "default";

/// The relations answered `NotYetRead` among `answers`, each as the line the run prints.
#[must_use]
pub fn listed(answers: &[(Relation, Option<AbsenceCause>)]) -> Vec<String> {
    answers
        .iter()
        .filter(|(_, reason)| *reason == Some(AbsenceCause::NotYetRead))
        .map(|(relation, _)| format!("not yet read: {}", folder_of(*relation)))
        .collect()
}

/// The relations of the catalog with the answer `answer` gives for each, the engine's own left out.
#[must_use]
pub fn answers(
    answer: impl Fn(Relation) -> Option<AbsenceCause>,
) -> Vec<(Relation, Option<AbsenceCause>)> {
    all()
        .iter()
        .copied()
        .filter(|relation| provenance_of(*relation) != Provenance::Engine)
        .map(|relation| (relation, answer(relation)))
        .collect()
}

/// Whether the language named `name` ships: the command line's manifest, found from the language's directory `directory`, lists it
/// among its default features (`[features] default`), however the manifest is formatted. A language outside the repository, whose
/// directory has no command line's manifest beside it, does not; a manifest that cannot be read or does not parse is an error.
fn ships(name: &str, directory: &Path) -> Result<bool, String> {
    let path = directory.join(COMMAND_LINE_MANIFEST);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("{}: cannot be read: {error}", path.display())),
    };
    let manifest = text
        .parse::<toml::Table>()
        .map_err(|error| format!("{}: does not parse: {error}", path.display()))?;
    Ok(manifest
        .get(FEATURES)
        .and_then(|features| features.get(DEFAULT_FEATURES))
        .and_then(toml::Value::as_array)
        .is_some_and(|default| default.iter().any(|each| each.as_str() == Some(name))))
}

/// Prints each relation the language has not read yet, by name, so that a stage of a language being written is never a silence:
/// the line is printed whether the run passes or not. Shipped baseline relations and advertised optional capabilities must be answered; unadvertised optional capabilities remain explicitly unavailable.
pub(super) fn report_not_yet_read(
    name: &str,
    directory: &Path,
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    findings: &mut Findings,
) {
    let observed = answers(|relation| {
        if provenance_of(relation) == Provenance::Manifests {
            language.absence_of(relation)
        } else {
            parse_absence(parse, relation)
        }
    });
    let shipped = match ships(name, directory) {
        Ok(shipped) => shipped,
        Err(why) => {
            findings.0.push(why);
            true
        }
    };
    let promises = parse.capabilities();
    if duplicates(promises) != 0 {
        findings
            .0
            .push(format!("duplicate advertised capabilities: {promises:?}"));
    }
    let mut errors = std::io::stderr().lock();
    for (relation, reason) in observed {
        if reason != Some(AbsenceCause::NotYetRead) {
            continue;
        }
        let line = format!("not yet read: {}", folder_of(relation));
        drop(writeln!(errors, "{line}"));
        let required = needs_answer(relation, promises, shipped);
        if required {
            findings.0.push(format!("{line}: {name} must answer its required baseline and advertised optional capabilities"));
        }
    }
}

/// Optional promises are required even when a language is not a default CLI feature.
fn needs_answer(relation: Relation, promises: &[OptionalCapability], shipped: bool) -> bool {
    ryo_relations::domain::relations::service::optional_capability(relation)
        .map_or(shipped, |capability| promises.contains(&capability))
}
/// Duplicate declarations are an invalid provider inventory, not extra support.
fn duplicates(promises: &[OptionalCapability]) -> usize {
    promises
        .iter()
        .enumerate()
        .filter(|(index, capability)| {
            promises
                .iter()
                .take(*index)
                .any(|previous| previous == *capability)
        })
        .count()
}

#[cfg(test)]
#[path = "not_yet_read_test.rs"]
mod tests;
