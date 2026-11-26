//! The judgments of the conformance, made on what the walk gave: the rows of each relation against the rows the language expects, the
//! relations the language does not have against the fixtures it keeps, the kinds against their samples.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::super::types::{ParseEngine, Sample};
use super::expected::{Cell, Expected, Fixtures};
use super::findings::{Findings, folders_in, input_in, inputs_in, read_as};
use super::rendered::shown as row_shown;
use super::rows::{check_input_identity, compare, rows_of_relation};
use super::samples::samples_in;
use super::walked::walk_file;
use crate::domain::files::types::SourcePath;
use crate::inbound::use_cases::project::types::ProjectEngine;
use crate::inbound::use_cases::walk::types::WalkEngine;
use ryo_relations::domain::relations::service::{all, folder_of, kinds, provenance_of};
use ryo_relations::domain::relations::types::{Provenance, Relation};

/// The engines a fixture is read with when its text declares a configuration of its own, or none.
pub type Engines = (std::sync::Arc<dyn ParseEngine>, std::sync::Arc<dyn WalkEngine>);

/// The configuration a fixture's text declares: the engines to read it with, none when it declares nothing, or why it is refused.
pub type Configuration<'a> = dyn Fn(&str) -> Result<Option<Engines>, String> + 'a;

/// The rows of `relation` that every input of the fixture gives, read through the walk, sorted.
fn rows_of_inputs(
    (parse, walk): (&dyn ParseEngine, &dyn WalkEngine),
    configure: &Configuration<'_>,
    inputs: &[PathBuf],
    relation: Relation,
    folder: &str,
    findings: &mut Findings,
) -> Option<Vec<Vec<Cell>>> {
    if !check_input_identity(inputs.len(), kinds(relation), folder, findings) {
        return None;
    }
    let mut paths = BTreeSet::new();
    let mut rows = Vec::new();
    for input in inputs {
        let name = input.file_name()?.to_string_lossy();
        let shown = read_as(&format!("{folder}/{name}"));
        if !paths.insert(shown.clone()) {
            findings.0.push(format!("{folder}: multiple inputs normalize to the same file {shown}"));
            return None;
        }
        let bytes = match std::fs::read(input) {
            Ok(bytes) => bytes,
            Err(error) => {
                findings.0.push(format!("{shown}: cannot read fixture input: {error}"));
                return None;
            }
        };
        let configured = match std::str::from_utf8(&bytes) {
            Ok(text) => match configure(text) {
                Ok(configured) => configured,
                Err(refusal) => {
                    findings.0.push(format!("{shown}: the configuration the fixture declares is refused: {refusal}"));
                    return None;
                }
            },
            // Binary fixture bytes keep the language's own engines.
            Err(_) => None,
        };
        let (parse, walk): (&dyn ParseEngine, &dyn WalkEngine) = configured.as_ref().map_or((parse, walk), |(parse, walk)| (parse.as_ref(), walk.as_ref()));
        if walk.recognizes(SourcePath(&shown), false) == crate::inbound::use_cases::parse::types::Recognition::Declines {
            findings.0.push(format!("{shown}: a fixture the language does not recognize: give it a path the language reads"));
            return None;
        }
        match walk_file(parse, walk, &shown, bytes) {
            Ok(file) => rows.extend(rows_of_relation(&file.read, relation, &shown)),
            Err(error) => {
                findings.0.push(format!("{shown}: a fixture the language fails to read: {error}"));
                return None;
            }
        }
    }
    rows.sort();
    Some(rows)
}

/// Judges the answer of the language for one relation read from trees.
pub(super) fn judge_syntax(engines: (&dyn ParseEngine, &dyn WalkEngine), configure: &Configuration<'_>, fixtures: &Path, relation: Relation, expected: &[Expected], findings: &mut Findings) {
    let walk = engines.1;
    let folder = folder_of(relation);
    let positive = fixtures.join("relations").join(folder);
    if walk.absence_of(relation).is_some() {
        if positive.exists() {
            findings.0.push(format!(
                "{folder}: answers NotInThisLanguage but tests/fixtures/relations/{folder}/ shows the construct: delete it, or answer Reads"
            ));
        }
        return;
    }
    let inputs = inputs_in(&positive);
    if inputs.is_empty() {
        findings.0.push(format!("{folder}: answers Reads but tests/fixtures/relations/{folder}/ has no input"));
        return;
    }
    let Some(written) = rows_of_inputs(engines, configure, &inputs, relation, &folder, findings) else { return };
    let mut wanted: Vec<Vec<Cell>> = expected.iter().filter(|each| each.relation == &*folder).flat_map(|each| each.rows.clone()).collect();
    wanted.sort();
    if wanted.is_empty() {
        findings.0.push(format!("{folder}: answers Reads but the language expects no row of it (`Expected` for `{folder}`)"));
    }
    compare(&format!("{folder}: tests/fixtures/relations/{folder}/"), &written, &wanted, &|row| row_shown(relation, row), findings);
}

/// Holds that a linked language is a language: it reads at least one relation, it has samples of what it reads and recognizes them, and
/// its trees have kinds.
pub(super) fn judge_presence(walk: &dyn WalkEngine, language: &dyn ProjectEngine, fixtures: &Path, findings: &mut Findings) {
    let reads = all().iter().any(|relation| {
        provenance_of(*relation) != Provenance::Engine
            && if provenance_of(*relation) == Provenance::Manifests {
                language.absence_of(*relation).is_none()
            } else {
                walk.absence_of(*relation).is_none()
            }
    });
    if !reads {
        findings.0.push("the language reads no relation: every relation is answered as not in it".to_owned());
    }
    if walk.kind_count().0 == 0 {
        findings.0.push("kind_count is 0: the trees of the language have no kinds".to_owned());
    }
    let names = walk.vocabulary().kinds.0;
    if names.len() != walk.kind_count().0 {
        findings.0.push(format!("the vocabulary lists {} kinds but kind_count is {}: each kind has one name", names.len(), walk.kind_count().0));
    }
    let distinct: BTreeSet<&str> = names.iter().map(|name| name.0).collect();
    if distinct.len() != names.len() {
        findings.0.push("the vocabulary lists a name twice: a name is one kind".to_owned());
    }
    let samples = samples_in(fixtures);
    if samples.is_empty() {
        findings.0.push("tests/fixtures has no sample file: a relation's input.<ext> or a file of allocation/".to_owned());
    }
    for sample in &samples {
        if walk.recognizes(sample.path, false) == crate::inbound::use_cases::parse::types::Recognition::Declines {
            findings.0.push(format!("{}: a sample the language does not recognize: give it a path the language reads", sample.path));
        }
    }
}

/// The extension of the language's files, from the input of the first relation it reads.
fn extension_of(fixtures: &Path) -> String {
    folders_in(&fixtures.join("relations"))
        .iter()
        .filter_map(|folder| input_in(&fixtures.join("relations").join(folder)))
        .find_map(|input| input.extension().map(|each| each.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// Holds every kind of the language's trees to its sample: the sample's tree holds an element of the kind, and there is a sample for each
/// kind or it is marked unread with a reason. (That no relation reads a kind with a sample is not held: the walk does not declare the
/// kinds its fillers look at.)
pub(super) fn judge_kinds(name: &str, parse: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path, expected: &Fixtures, findings: &mut Findings) {
    let declared: BTreeSet<u16> = (0..walk.kind_count().0).filter_map(|number| u16::try_from(number).ok()).collect();
    let shown: BTreeSet<u16> = expected.kinds.iter().map(|each| each.bits.0).collect();
    if shown.len() != expected.kinds.len() {
        findings.0.push(format!(
            "the language shows {} kinds but only {} of them are different: each kind has one sample or one Sample::Unread",
            expected.kinds.len(),
            shown.len()
        ));
    }
    for number in declared.difference(&shown) {
        findings.0.push(format!("kind {number} of the {} the language declares has no sample: show it with a sample or Sample::Unread", declared.len()));
    }
    for number in shown.difference(&declared) {
        findings.0.push(format!("kind {number} is shown but the language declares only {} kinds (kind_count)", declared.len()));
    }
    for each in &expected.kinds {
        if let Sample::Unread(reason) = each.sample
            && reason.is_empty()
        {
            findings.0.push(format!("{name}: NodeKind::{} is marked Sample::Unread(\"\") with no reason: say why no relation models it", each.kind));
        }
    }
    let path = format!("kinds/sample.{}", extension_of(fixtures));
    for each in &expected.kinds {
        let (source, token) = match each.sample {
            Sample::Node(source) => (source, false),
            Sample::Token(source) => (source, true),
            Sample::Unread(_) => continue,
        };
        let walked = walk_file(parse, walk, &path, source.as_bytes().to_vec());
        let held = walked.as_ref().is_ok_and(|file| file.elements.iter().any(|(kind, is_token)| *kind == each.bits.0 && *is_token == token));
        if let Ok(file) = &walked
            && let Some((outside, _)) = file.elements.iter().find(|(number, _)| !declared.contains(number))
        {
            findings.0.push(format!(
                "the sample of kind {} holds an element of kind {outside}, which the language does not declare (kind_count is {})",
                each.bits.0,
                declared.len()
            ));
        }
        if !held {
            let present = walked.as_ref().map_or_else(
                |error| format!("the walk refuses the sample: {error}"),
                |file| {
                    let mut present: Vec<String> = file.elements.iter().map(|(kind, is_token)| format!("{kind}{}", if *is_token { "t" } else { "n" })).collect();
                    present.sort();
                    present.dedup();
                    format!("its table holds kinds {}", present.join(" "))
                },
            );
            findings.0.push(format!(
                "{name}: the sample of `{}` = kind {} ({}) has no {} of that kind: {present}",
                each.kind,
                each.bits.0,
                source.escape_debug(),
                if token { "token" } else { "node" }
            ));
        }
    }
}
