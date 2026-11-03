//! Conflicting failures exercise the original metadata and empty-result boundaries.
use super::execution::CommonOperatorEngine;
use crate::domain::features::service::kind_of;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::service::registered;
use crate::domain::table_model::{
    errors::TableModelError,
    types::{RowCount, Rows, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{
    relations::{
        service::node_id,
        types::{CellKind, FileId, Integer, Position, Relation, Value},
    },
    symbols::types::Symbols,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction, Input, RepeatedRows},
    plans::types::Operation,
};
fn collect(operation: &Operation, tables: &[Table]) -> Result<Table, OperatingError> {
    let schemas = tables
        .iter()
        .map(|table| InputSchema {
            kinds: &table.kinds,
            tree: None,
        })
        .collect::<Vec<_>>();
    let contract = super::derived::prepare::operator(
        registered(kind_of(operation)),
        operation,
        &schemas,
        &[],
    )?;
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let inputs = tables.iter().map(Rows::Computed).collect::<Vec<_>>();
    let mut output = Table::default();
    CommonOperatorEngine.collect(&contract, &inputs, &mut context, &mut output)?;
    Ok(output)
}
fn project(column: u32) -> Operation {
    Operation::Project {
        from: Input::Relation(Relation::Function),
        columns: vec![ColumnIndex(column)],
        distinct: RepeatedRows::Kept,
    }
}
fn numbers(malformed: bool) -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: if malformed {
            vec![Value::Number(Integer(1))]
        } else {
            vec![Value::Number(Integer(1)), Value::Number(Integer(2))]
        },
        count: RowCount(2),
    }
}
#[test]
fn project_static_column_failure_precedes_malformed_population() {
    for malformed in [false, true] {
        assert_eq!(
            collect(&project(99), &[numbers(malformed)]).err(),
            Some(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99
            }))
        );
    }
    // The same malformed population must still fail when the metadata is valid.
    assert_eq!(
        collect(&project(0), &[numbers(true)]).err(),
        Some(OperatingError::ShapeMismatch)
    );
    let valid = collect(&project(0), &[numbers(false)]).unwrap();
    assert_eq!(valid.count.0, 2);
    assert_eq!(
        valid.cells,
        vec![Value::Number(Integer(1)), Value::Number(Integer(2))]
    );
}
fn node(position: u32) -> Value {
    Value::Node(node_id(FileId(17), Position(position)))
}
fn axis(direction: Direction, id: u32) -> Operation {
    Operation::Axis {
        from: Input::Relation(Relation::Function),
        edges: Input::Relation(Relation::Scope),
        domain: Input::Relation(Relation::SyntaxTree),
        direction,
        id: ColumnIndex(id),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    }
}
fn axis_inputs(empty_roots: bool, empty_domain: bool) -> Vec<Table> {
    vec![
        Table {
            kinds: vec![CellKind::Node],
            cells: if empty_roots { vec![] } else { vec![node(1)] },
            count: RowCount(usize::from(!empty_roots)),
        },
        Table {
            kinds: vec![CellKind::Node; 2],
            cells: vec![node(1), node(2)],
            count: RowCount(1),
        },
        Table {
            kinds: vec![CellKind::Node],
            cells: if empty_domain { vec![] } else { vec![node(2)] },
            count: RowCount(usize::from(!empty_domain)),
        },
    ]
}
#[test]
fn collected_axis_empty_roots_do_not_acquire_unsupported_table_walk() {
    let empty = collect(&axis(Direction::Deep, 0), &axis_inputs(true, false)).unwrap();
    assert_eq!(empty.kinds, vec![CellKind::Node; 2]);
    assert_eq!(empty.count.0, 0);
    assert!(empty.cells.is_empty());
    // Root-id metadata is checked before the original empty-root gate.
    assert_eq!(
        collect(&axis(Direction::Deep, 99), &axis_inputs(true, false)).err(),
        Some(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 99
        }))
    );
    // A real nonempty unsupported walk must not become an unconditional no-op.
    assert_eq!(
        collect(&axis(Direction::Deep, 0), &axis_inputs(false, false)).err(),
        Some(OperatingError::WrongOperation)
    );
    let child = collect(&axis(Direction::Child, 0), &axis_inputs(false, false)).unwrap();
    assert_eq!(child.count.0, 1);
    assert_eq!(child.cells, vec![node(1), node(2)]);
}
#[test]
fn collected_axis_computed_empty_domain_keeps_its_original_shortcut() {
    let empty = collect(&axis(Direction::Deep, 0), &axis_inputs(false, true)).unwrap();
    assert_eq!(empty.kinds, vec![CellKind::Node; 2]);
    assert_eq!(empty.count.0, 0);
    assert_eq!(
        collect(&axis(Direction::Deep, 99), &axis_inputs(false, true)).err(),
        Some(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 99
        }))
    );
}
