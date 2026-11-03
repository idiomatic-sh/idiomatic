//! The `repository` service: free functions named by verb over the files of a run, with no IO and no ports.

use crate::domain::keeping::service::configuration_bytes;
use crate::domain::keeping::types::{Configuration, DigestInput, HostedText};
use crate::domain::rules::types::RuleSpec;
use ryo_language::domain::files::types::FilePath;
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
        .map(|relation| {
            (
                name(*relation).0,
                columns(*relation)
                    .into_iter()
                    .map(|column| (column_name(column).0, kind(column)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    schema_bytes(&relations)
}
fn schema_bytes(
    relations: &[(
        &str,
        Vec<(&str, ryo_relations::domain::relations::types::CellKind)>,
    )],
) -> DigestInput {
    use ryo_relations::domain::relations::types::CellKind;
    fn number(bytes: &mut Vec<u8>, value: usize) {
        bytes.extend_from_slice(&(value as u64).to_le_bytes());
    }
    fn frame(bytes: &mut Vec<u8>, value: &[u8]) {
        number(bytes, value.len());
        bytes.extend_from_slice(value);
    }
    let mut bytes = Vec::new();
    frame(&mut bytes, b"ryo-relation-schema-v1");
    number(&mut bytes, relations.len());
    for (relation, columns) in relations {
        frame(&mut bytes, relation.as_bytes());
        number(&mut bytes, columns.len());
        for (column, kind) in columns {
            frame(&mut bytes, column.as_bytes());
            bytes.push(match kind {
                CellKind::Node => 0,
                CellKind::Symbol => 1,
                CellKind::File => 2,
                CellKind::Span => 3,
                CellKind::Number => 4,
                CellKind::Flag => 5,
            });
        }
    }
    DigestInput(bytes)
}
#[cfg(test)]
mod schema_identity_tests {
    use super::*;
    use ryo_relations::domain::relations::types::CellKind;
    #[test]
    fn names_kinds_and_column_order_are_part_of_snapshot_identity() {
        let original = [(
            "module",
            vec![("file", CellKind::File), ("path", CellKind::Symbol)],
        )];
        let same = [(
            "module",
            vec![("file", CellKind::File), ("path", CellKind::Symbol)],
        )];
        assert_eq!(schema_bytes(&original), schema_bytes(&same));
        for changed in [
            vec![(
                "renamed",
                vec![("file", CellKind::File), ("path", CellKind::Symbol)],
            )],
            vec![(
                "module",
                vec![("file", CellKind::File), ("renamed", CellKind::Symbol)],
            )],
            vec![(
                "module",
                vec![("file", CellKind::File), ("path", CellKind::Number)],
            )],
            vec![(
                "module",
                vec![("path", CellKind::Symbol), ("file", CellKind::File)],
            )],
            vec![(
                "module",
                vec![
                    ("file", CellKind::File),
                    ("path", CellKind::Symbol),
                    ("extra", CellKind::Symbol),
                ],
            )],
        ] {
            assert_ne!(schema_bytes(&original), schema_bytes(&changed));
        }
    }
    #[test]
    fn current_module_context_schema_rejects_its_legacy_two_column_identity() {
        use ryo_relations::domain::relations::{
            service::{column_name, columns, kind},
            types::Relation,
        };
        let legacy = [(
            "module-identity",
            vec![("file", CellKind::File), ("path", CellKind::Symbol)],
        )];
        let current = [(
            "module-identity",
            columns(Relation::ModuleIdentity)
                .into_iter()
                .map(|column| (column_name(column).0, kind(column)))
                .collect::<Vec<_>>(),
        )];
        assert_ne!(schema_bytes(&legacy), schema_bytes(&current));
        let previous = [(
            "module-identity",
            vec![
                ("file", CellKind::File),
                ("path", CellKind::Symbol),
                ("module-name", CellKind::Symbol),
                ("package-name", CellKind::Symbol),
            ],
        )];
        assert_ne!(schema_bytes(&previous), schema_bytes(&current));
    }
    #[test]
    fn frames_distinguish_adjacent_names_and_real_catalog_is_stable() {
        assert_ne!(
            schema_bytes(&[("ab", vec![("c", CellKind::Number)])]),
            schema_bytes(&[("a", vec![("bc", CellKind::Number)])])
        );
        assert_eq!(catalog_schema(), catalog_schema());
        assert!(!catalog_schema().0.is_empty());
    }
}

/// Stable, length-delimited request selection identity; computed once per request.
pub(crate) fn source_selection_identity(
    name: Option<&ryo_values::domain::descriptions::types::LanguageName>,
) -> DigestInput {
    let mut bytes = b"ryo-source-selection-v1".to_vec();
    match name {
        None => bytes.push(0),
        Some(name) => {
            bytes.push(1);
            bytes.extend_from_slice(
                &u64::try_from(name.0.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(name.0.as_bytes());
        }
    }
    DigestInput(bytes)
}
