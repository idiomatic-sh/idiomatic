//! Schema-derived typed populations and ordinary access, shared by admission fixtures.
use crate::domain::table_model::{
    service::{count_of, read_row},
    types::{Rows, SelectedRows, StoredSelection},
};
use ryo_relations::domain::relations::{
    service::{node_id, to_bits},
    types::{
        CellKind, ColumnsView, FactColumn, FileId, Flag, Integer, Offset, Position, Relation, Span,
        Value,
    },
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

macro_rules! catalog_groups {
    (syntax { $( $syntax:ident $sm:ident $sr:ident $sc:ident ; )* }
     manifest { $( $manifest:ident $mm:ident $mr:ident $mc:ident ; )* }
     engine { $( $engine:ident $em:ident $er:ident $ec:ident ; )* }) => {
        pub(super) fn group(relation: Relation) -> usize {
            match relation { $(Relation::$syntax => 0,)* $(Relation::$manifest => 1,)* $(Relation::$engine => 2,)* }
        }
    };
}
ryo_relations::each_relation!(catalog_groups);

pub(super) fn row(schema: &[CellKind], at: u32) -> Vec<Value> {
    schema
        .iter()
        .enumerate()
        .map(|(column, kind)| {
            let column = u32::try_from(column).unwrap();
            match kind {
                CellKind::Node => Value::Node(node_id(FileId(9), Position(at + column + 1))),
                CellKind::Symbol => Value::Symbol(Symbol(at + column + 3)),
                CellKind::File => Value::File(FileId(9)),
                CellKind::Span => Value::Span(Span {
                    start: Offset((at + column) * 2),
                    end: Offset((at + column) * 2 + 2),
                }),
                CellKind::Number => Value::Number(Integer(i64::from(100 + at * 10 + column))),
                CellKind::Flag => Value::Flag(if at == 0 { Flag::Raised } else { Flag::Lowered }),
            }
        })
        .collect()
}
pub(super) fn columns(schema: &[CellKind]) -> Vec<FactColumn> {
    let rows = [row(schema, 0), row(schema, 1)];
    (0..schema.len())
        .map(|at| FactColumn(rows.iter().map(|row| to_bits(row[at]).0).collect()))
        .collect()
}
pub(super) fn read(rows: &Rows<'_>) -> Vec<Vec<Value>> {
    let mut decode = Vec::new();
    (0..count_of(rows).0)
        .map(|at| {
            read_row(
                rows,
                ryo_values::domain::rows::types::RowIndex(at),
                &mut decode,
            )
            .unwrap()
            .to_vec()
        })
        .collect()
}
pub(super) fn selected<'a>(
    source: ColumnsView<'a>,
    positions: Vec<u32>,
    columns: Vec<ColumnIndex>,
    kinds: Vec<CellKind>,
) -> SelectedRows<'a> {
    SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: Relation::SourceLocation,
            rows: crate::domain::stored_selection::types::SelectedPositions(Arc::new(positions)),
            columns,
            kinds,
            identity: Arc::new(Default::default()),
        }),
    }
}
