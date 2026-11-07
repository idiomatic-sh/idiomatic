//! The `repository` service: free functions named by verb over the files of a run, with no IO and no ports.

use super::constants::{RELATION_SCHEMA_LABEL, SOURCE_SELECTION_LABEL};
use super::types::{DigestColumn, DigestRelation, SchemaCount};
use crate::domain::keeping::service::configuration_bytes;
use crate::domain::keeping::types::{Configuration, DigestInput, DigestPart, HostedText};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::CorpusInput;

/// What is read of the rules and the relations the host declares (as their lines, spelled once per run however many rules
/// are digested) to say whether what the store kept of the rules can be used, as the bytes to digest.
#[must_use]
pub(crate) fn configuration(rules: &[RuleSpec], hosted: &[HostedText]) -> DigestInput {
    configuration_bytes(&Configuration { rules, hosted })
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
