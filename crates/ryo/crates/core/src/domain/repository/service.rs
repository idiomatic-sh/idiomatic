//! The `repository` service: free functions named by verb over the files of a run, with no IO and no ports.

use super::constants::{COMPILED_PLAN_LABEL, RELATION_SCHEMA_LABEL, SOURCE_SELECTION_LABEL};
use super::types::{DigestColumn, DigestRelation, IdentityHasher, SchemaCount};
use crate::domain::keeping::service::{configuration_bytes, rule_head, rule_modules, rule_tail};
use crate::domain::keeping::types::{Configuration, DigestInput, DigestPart, HostedText};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::RuleSpec;

/// What is read of the rules and the relations the host declares (as their lines, spelled once per run however many rules
/// are digested) to say whether what the store kept of the rules can be used, as the bytes to digest.
#[must_use]
pub(crate) fn configuration(rules: &[RuleSpec], hosted: &[HostedText]) -> DigestInput {
    configuration_bytes(&Configuration { rules, hosted })
}

/// What a run was given of its rules, as bytes to digest before anything is compiled: every selected rule in order, as
/// `configuration` writes one (its id, its applicability, its root document, its parameters as written, what it observes, its
/// globs), but with the modules its query names written once for the run: a rule says which of the run's catalogs of modules is
/// its own, and each catalog follows the rules. The rules of a pack share one catalog, so the fragments are read once, not once
/// a rule. The same rules give the same bytes however they share their catalogs: a catalog equal to an earlier one is that one.
#[must_use]
pub(crate) fn given(rules: &[RuleSpec]) -> DigestInput {
    let mut bytes = DigestInput::default();
    count(&mut bytes, SchemaCount(rules.len()));
    // A rule of each catalog met so far: rules that share a catalog share its allocation, which is compared first, so the
    // modules' texts are read only where the allocations differ.
    let mut catalogs: Vec<&RuleSpec> = Vec::new();
    for rule in rules {
        rule_head(&mut bytes, rule);
        let met = catalogs
            .iter()
            .position(|held| held.query.modules == rule.query.modules);
        let at = met.unwrap_or_else(|| {
            catalogs.push(rule);
            catalogs.len().saturating_sub(1)
        });
        count(&mut bytes, SchemaCount(at));
        rule_tail(&mut bytes, rule);
    }
    count(&mut bytes, SchemaCount(catalogs.len()));
    for catalog in catalogs {
        rule_modules(&mut bytes, catalog);
    }
    bytes
}

/// Puts the files in the order of their paths, which is the order their findings leave in: a file is numbered by its place in it.
pub(crate) fn by_path(files: &mut [CorpusInput]) {
    files.sort_by(|left, right| input_path(left).cmp(input_path(right)));
}

/// The paths of the files, as the tables of the run spell them, in the order of the files.
#[must_use]
pub(crate) fn paths_of(files: &[CorpusInput]) -> Vec<FilePath> {
    files.iter().map(|file| input_path(file).clone()).collect()
}

/// A deterministic, versioned catalog description invalidates retained rows across schema changes.
/// Column order is semantic; length framing prevents adjacent names from aliasing.
#[must_use]
pub(crate) fn catalog_schema() -> DigestInput {
    use ryo_relations::domain::relations::service::{all, column_name, columns, kind, name};
    let relations = all()
        .iter()
        .map(|relation| DigestRelation {
            name: name(*relation),
            columns: columns(*relation)
                .into_iter()
                .map(|column| DigestColumn {
                    name: column_name(column),
                    kind: kind(column),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    schema_bytes(&relations)
}

/// Writes a count as eight bytes, least significant first.
fn count(into: &mut DigestInput, value: SchemaCount) {
    into.0
        .extend_from_slice(&u64::try_from(value.0).unwrap_or(u64::MAX).to_le_bytes());
}

/// Writes a piece of text with its length before it, so adjacent names cannot alias.
fn frame(into: &mut DigestInput, text: DigestPart<'_>) {
    count(into, SchemaCount(text.0.len()));
    into.0.extend_from_slice(text.0.as_bytes());
}

/// The bytes of a catalog: each relation's name and, in order, each column's name and kind, each framed by its length.
#[must_use]
pub(crate) fn schema_bytes(relations: &[DigestRelation<'_>]) -> DigestInput {
    let mut bytes = DigestInput::default();
    frame(&mut bytes, DigestPart(RELATION_SCHEMA_LABEL));
    count(&mut bytes, SchemaCount(relations.len()));
    for relation in relations {
        frame(&mut bytes, DigestPart(relation.name.0));
        count(&mut bytes, SchemaCount(relation.columns.len()));
        for column in &relation.columns {
            frame(&mut bytes, DigestPart(column.name.0));
            bytes.0.push(match column.kind {
                CellKind::Node => 0,
                CellKind::Symbol => 1,
                CellKind::File => 2,
                CellKind::Span => 3,
                CellKind::Number => 4,
                CellKind::Flag => 5,
            });
        }
    }
    bytes
}

/// Stable, length-delimited request selection identity; computed once per request.
pub(crate) fn source_selection_identity(
    name: Option<&ryo_values::domain::descriptions::types::LanguageName>,
) -> DigestInput {
    let mut bytes = DigestInput(SOURCE_SELECTION_LABEL.as_bytes().to_vec());
    match name {
        None => bytes.0.push(0),
        Some(name) => {
            bytes.0.push(1);
            bytes.0.extend_from_slice(
                &u64::try_from(name.0.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            bytes.0.extend_from_slice(name.0.as_bytes());
        }
    }
    bytes
}

/// The identity of the plan a rule executes: the lowered plan after the handover rewrites (regrouping, narrowing, pushing), not
/// its source text, as the label then the bytes the plan's derived `Hash` feeds. The derive covers every field, those added later
/// too, so a build whose rewrites put other steps at a frontier position, or keep other columns, names another plan, and what a
/// store kept under the old one is not used again.
#[must_use]
pub(crate) fn plan_identity(plan: &Plan) -> DigestInput {
    use std::hash::Hash as _;
    let mut hasher = IdentityHasher(DigestInput(COMPILED_PLAN_LABEL.as_bytes().to_vec()));
    plan.hash(&mut hasher);
    hasher.0
}
