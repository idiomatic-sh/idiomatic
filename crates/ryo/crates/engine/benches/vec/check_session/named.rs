//! Resolve file-scoped declarations against the linked providers before preparing the
//! program. The selected reader must still supply the table on each admitted file.
use super::{FileId, FileSource, Result, Run, Table, finish, l, reader_run::packet::FileSymbols, source::FileRelation};
use std::{collections::{BTreeMap, HashMap}, rc::Rc};

#[derive(Default)]
pub(crate) struct Catalog(BTreeMap<&'static str, &'static FileRelation>);

impl Catalog {
    pub(crate) fn from_readers<L: FileSource>(readers: &[L]) -> Result<Self> {
        let mut catalog = Self::default();
        for schema in readers.iter().flat_map(FileSource::declarations) {
            if schema.columns.is_empty() || schema.name.is_empty() {
                return Err(format!("empty file relation declaration {}", schema.name));
            }
            if ryo_relations::domain::relations::service::relation_named(
                ryo_relations::domain::labels::types::NameRef(schema.name),
            )
            .is_some()
                || catalog
                    .0
                    .insert(schema.name, schema)
                    .is_some_and(|previous| previous != schema)
            {
                return Err(format!("conflicting file relation {}", schema.name));
            }
        }
        Ok(catalog)
    }

    pub(crate) fn declare(&self, run: &mut Run) {
        for schema in self.0.values() {
            if run.catalog(schema.name).is_none() {
                declare(run, schema);
            }
        }
    }
}
pub(super) fn declare(run: &mut Run, schema: &FileRelation) {
    let columns: Vec<_> = schema
        .columns
        .iter()
        .map(|&(name, kind)| (name, finish::ty_of(kind)))
        .collect();
    run.declare(schema.name, &columns, finish::Supplier::Provider);
}

/// Empty rows have no file-local cells. Share their immutable table across
/// visits in this reader run after validating each supplier response.
#[derive(Default)]
pub(super) struct EmptyTables(rustc_hash::FxHashMap<&'static str, Rc<Table>>);

pub(super) fn table(
    empty: &mut EmptyTables,
    mut columns: super::source::Columns,
    schema: &FileRelation,
    id: FileId,
    symbols: &l::Symbols,
    bytes: &[u8],
    run: &mut Run,
    unknown: &HashMap<u32, u32>,
    resolved: &mut FileSymbols,
) -> Result<Rc<Table>> {
    let n = columns.first().map_or(0, |c| c.0.len());
    if columns.len() != schema.columns.len() || columns.iter().any(|c| c.0.len() != n) {
        return Err(format!(
            "provider returned invalid dimensions for {}",
            schema.name
        ));
    }
    if n == 0 && let Some(table) = empty.0.get(schema.name) {
        return Ok(table.clone());
    }
    let (_, names, kinds) = run
        .catalog(schema.name)
        .ok_or_else(|| format!("missing schema {}", schema.name))?;
    // The supplier already transferred its columns to this file visit.
    // Normalize the qualified cells in the same buffers.
    for (column, (_, kind)) in columns.iter_mut().zip(schema.columns) {
        match kind {
            l::CellKind::Symbol => {
                for word in column.0.iter_mut().filter(|word| **word != finish::NULL) {
                    let symbol = l::Symbol((*word).try_into().map_err(|_| "invalid symbol id")?);
                    *word = u64::from(resolved.resolve(run, unknown, symbols, bytes, symbol));
                }
            }
            l::CellKind::Node => {
                for node in column.0.iter_mut().filter(|node| **node != finish::NULL) {
                    if *node >> 32 != u64::from(id.0) {
                        return Err(format!("foreign node in file relation {}", schema.name));
                    }
                    *node = u64::from(*node as u32);
                }
            }
            _ => {}
        }
    }
    let table = Rc::new(Table { names, tys: kinds, cols: columns.into_iter().map(|column| column.0).collect(), n });
    if n == 0 {
        empty.0.insert(schema.name, table.clone());
    }
    Ok(table)
}
