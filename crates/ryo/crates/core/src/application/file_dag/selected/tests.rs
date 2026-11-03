use super::SelectedInput;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::replay::types::Collect;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::take_read_row_counts,
    types::{SelectedRows, StoredSelection, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value},
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use std::sync::Arc;

fn source(values: &[&[i64]]) -> Vec<FactColumn> {
    values
        .iter()
        .map(|column| {
            FactColumn(
                column
                    .iter()
                    .map(|value| to_bits(Value::Number(Integer(*value))).0)
                    .collect(),
            )
        })
        .collect()
}

fn selected<'a>(columns: &'a [FactColumn], positions: Vec<u32>) -> SelectedRows<'a> {
    SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number, CellKind::Number],
            columns,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(positions),
            columns: vec![ColumnIndex(1), ColumnIndex(0)],
            kinds: vec![CellKind::Number, CellKind::Number],
            identity: Arc::new(Default::default()),
        }),
    }
}

#[test]
fn selected_join_keeps_duplicate_bag_order_and_decodes_only_projected_cells()
-> Result<(), OperatingError> {
    let columns = source(&[&[1, 4, 3], &[7, 8, 7]]);
    let selected = SelectedInput::prepare(&selected(&columns, vec![2, 0, 2]), &[ColumnIndex(0)])?;
    assert_eq!(
        selected
            .view()
            .matching(&[Value::Number(Integer(7))])
            .collect::<Vec<_>>(),
        vec![RowIndex(0), RowIndex(1), RowIndex(2)]
    );
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let left = [Value::Number(Integer(7)), Value::Number(Integer(99))];
    let output = [
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(1),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
    ];
    let mut table = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let _previous = take_read_row_counts();
    selected.emit_join(
        &super::super::execution::PrimitiveSemanticsBridge {
            port: crate::outbound::operators::factory::test_port(),
        },
        &[ColumnIndex(0)],
        &output,
        &left,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut context,
        &mut Collect(&mut table),
    )?;
    assert_eq!(take_read_row_counts(), (0, 0));
    assert_eq!(table.count.0, 3);
    assert_eq!(
        table.cells,
        [
            Value::Number(Integer(99)),
            Value::Number(Integer(3)),
            Value::Number(Integer(3)),
            Value::Number(Integer(99)),
            Value::Number(Integer(1)),
            Value::Number(Integer(1)),
            Value::Number(Integer(99)),
            Value::Number(Integer(3)),
            Value::Number(Integer(3)),
        ]
    );
    Ok(())
}

#[test]
fn malformed_selected_populations_are_refused_before_indexing() {
    let complete = source(&[&[1, 2], &[7, 8]]);
    let mut ragged = complete.clone();
    ragged[1].0.pop();
    assert!(matches!(
        SelectedInput::prepare(&selected(&ragged, vec![0]), &[ColumnIndex(0)]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        SelectedInput::prepare(&selected(&complete, vec![2]), &[ColumnIndex(0)]),
        Err(OperatingError::ShapeMismatch)
    ));
}
