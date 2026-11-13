//! R4, the scope's first risk: a relation built alone, on demand, in any order, holds what the parse step's one eager walk of every
//! relation writes. Readers intern names in walk order, so the symbol numbers may differ; the texts they stand for must not.

use crate::fixtures::{parse, python};
use ryo_evaluation::outbound::facts::interface::Facts as _;
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::{from_bits, kinds};
use ryo_relations::domain::relations::types::{Bits, FactColumn, FileId, Relation, Value};

/// The relations Stage A and Stage B read through the facts port.
pub(crate) const LAZY: [Relation; 4] = [
    Relation::SourceLocation,
    Relation::QualifiedName,
    Relation::Comment,
    Relation::IndentRegion,
];

/// The engine's Python relation fixtures, as path and bytes.
pub(crate) fn engine_fixtures() -> Vec<(String, Vec<u8>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../languages/python/tests/fixtures/relations");
    let mut found: Vec<_> = std::fs::read_dir(&root)
        .expect("the fixtures")
        .filter_map(|entry| {
            let directory = entry.ok()?.path();
            let input = directory.join("input.py");
            let bytes = std::fs::read(&input).ok()?;
            Some((
                format!("{}/input.py", directory.file_name()?.to_str()?),
                bytes,
            ))
        })
        .collect();
    found.sort();
    found
}

/// The rows of one relation's columns, each value rendered with its text; and its raw values.
fn rows(kinds: &[ryo_relations::domain::relations::types::CellKind], columns: &[FactColumn], symbols: &Symbols, source: &[u8]) -> (Vec<Vec<String>>, Vec<Vec<Value>>) {
    let count = columns.first().map_or(0, |column| column.0.len());
    let mut rendered = Vec::new();
    let mut raw = Vec::new();
    for at in 0..count {
        let values: Vec<Value> = columns
            .iter()
            .zip(kinds)
            .map(|(column, kind)| from_bits(*kind, Bits(column.0[at])))
            .collect();
        rendered.push(
            values
                .iter()
                .map(|value| match value {
                    Value::Symbol(symbol) => format!(
                        "{:?}",
                        ryo_language::domain::symbols::service::text_of(symbols, ByteSlice(source), *symbol)
                            .map(|text| text.0.to_owned())
                    ),
                    other => format!("{other:?}"),
                })
                .collect(),
        );
        raw.push(values);
    }
    (rendered, raw)
}

#[test]
fn lazy_relations_hold_what_the_eager_walk_writes_in_any_order() {
    let language = python();
    let mut identical_numbers = 0;
    let mut renumbered = Vec::new();
    for (place, (path, bytes)) in engine_fixtures().iter().enumerate() {
        let file = FileId(u32::try_from(place).expect("a number"));
        let (mut eager_elements, mut eager_symbols) = parse(&*language, path, bytes, file);
        let mut eager: Vec<Vec<FactColumn>> = LAZY
            .iter()
            .map(|relation| vec![FactColumn::default(); kinds(*relation).len()])
            .collect();
        {
            let mut writer: ColumnsMut<'_> = LAZY
                .iter()
                .copied()
                .zip(eager.iter_mut().map(Vec::as_mut_slice))
                .collect();
            ryo_language::inbound::use_cases::parse::service::walk(
                &*language,
                &mut eager_elements,
                &mut eager_symbols,
                ByteSlice(bytes),
                file,
                &LAZY,
                &mut writer,
            );
        }
        let (mut lazy_elements, mut lazy_symbols) = parse(&*language, path, bytes, file);
        let facts = ryo_evaluation::outbound::facts::factory::file(&*language, &mut lazy_elements, bytes, file);
        let mut lazy = Vec::new();
        for relation in LAZY.iter().rev() {
            let view = facts.relation(*relation, &mut lazy_symbols).expect("the relation");
            lazy.push((*relation, view.columns.to_vec()));
        }
        drop(facts);
        for (relation, columns) in lazy {
            let at = LAZY.iter().position(|each| *each == relation).expect("a lazy relation");
            let (eager_rows, eager_raw) = rows(kinds(relation), &eager[at], &eager_symbols, bytes);
            let (lazy_rows, lazy_raw) = rows(kinds(relation), &columns, &lazy_symbols, bytes);
            let first = lazy_rows.iter().zip(&eager_rows).position(|(one, other)| one != other);
            assert_eq!(
                lazy_rows,
                eager_rows,
                "{relation:?} of {path}: {} lazy rows, {} eager rows, first difference at {first:?}: {:?} / {:?}",
                lazy_rows.len(),
                eager_rows.len(),
                first.and_then(|at| lazy_rows.get(at)),
                first.and_then(|at| eager_rows.get(at)),
            );
            if lazy_raw == eager_raw {
                identical_numbers += 1;
            } else {
                renumbered.push(format!("{relation:?} of {path}"));
            }
        }
    }
    println!("R4: relation populations with identical symbol numbers: {identical_numbers}; renumbered: {renumbered:?}");
}
