//! Resolve file-scoped declarations against the linked providers before preparing the
//! program. The selected reader must still supply the table on each admitted file.
use super::{FileId, FileSource, Result, Run, Table, finish, l, reader, source::FileRelation};
use std::collections::{BTreeMap, HashMap};

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
            declare(run, schema);
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

pub(super) fn table<L: FileSource>(
    source: &L,
    schema: &FileRelation,
    id: FileId,
    file: &L::File,
    tree: &l::NodeTable,
    symbols: &mut L::Dictionary,
    bytes: &[u8],
    run: &mut Run,
    unknown: &HashMap<u32, u32>,
) -> Result<Table> {
    let columns = source.named_rows(schema, id, file, tree, bytes, symbols)?;
    let n = columns.first().map_or(0, |c| c.0.len());
    if columns.len() != schema.columns.len() || columns.iter().any(|c| c.0.len() != n) {
        return Err(format!(
            "provider returned invalid dimensions for {}",
            schema.name
        ));
    }
    let (_, names, kinds) = run
        .catalog(schema.name)
        .ok_or_else(|| format!("missing schema {}", schema.name))?;
    let mut result = Table::with_rows(names, kinds, n);
    for ((out, column), (_, kind)) in result.cols.iter_mut().zip(columns).zip(schema.columns) {
        for raw in column.0 {
            let value = match kind {
                _ if raw == finish::NULL => raw,
                l::CellKind::Symbol => u64::from(reader::symbol(
                    run,
                    unknown,
                    L::symbols(symbols),
                    bytes,
                    l::Symbol(raw.try_into().map_err(|_| "invalid symbol id")?),
                )),
                l::CellKind::Node if raw >> 32 == u64::from(id.0) => u64::from(raw as u32),
                l::CellKind::Node => {
                    return Err(format!("foreign node in file relation {}", schema.name));
                }
                _ => raw,
            };
            out.push(value);
        }
    }
    result.n = n;
    Ok(result)
}
