//! Conflicting metadata, population and resource failures through the actual execution owner.
use super::super::super::fixtures;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::domain::operating::errors::OperatingError;
use ryo_language::{domain::elements::types::Elements, domain::symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::node_id,
    types::{CellKind, FileId, Integer, Position, Relation, Value as CellValue},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Direction, Input as PlanInput, RepeatedRows},
        plans::types::Operation,
        rows::{
            errors::RowsError,
            types::{RowCount, Table},
        },
    },
};

/// A genuine provider population: malformed flat rows must not pass through hosted-row conversion.
fn project_result(operation: &Operation, malformed: bool) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let symbol = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ryo_language::domain::symbols::types::ByteSlice(b""),
        ryo_relations::domain::terms::types::NameText("function"),
    );
    let table = functions(malformed, symbol);
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let (engine, prepared) =
        fixtures::prepared(fixtures::plan(vec![operation.clone()]), vec![], vec![])?;
    fixtures::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut symbols,
        &read,
    )
    .map(|(table, _)| table)
}
/// Canonical facts and declared projections reach the same mandatory owner, not a primitive harness.
fn axis_result(operation: &Operation, tables: &[Table]) -> Result<Table, OperatingError> {
    let references = tables.iter().collect::<Vec<_>>();
    let mut symbols = Symbols::default();
    fixtures::table_request(operation.clone(), &references, vec![], &mut symbols)
        .map(|(table, _)| table)
}

fn project(column: u32) -> Operation {
    Operation::Project {
        from: PlanInput::Relation(Relation::Function),
        columns: vec![ColumnIndex(column)],
        distinct: RepeatedRows::Kept,
    }
}
/// Advertise the real catalog schema; only the malformed case truncates physical storage.
fn functions(malformed: bool, symbol: ryo_relations::domain::terms::types::Symbol) -> Table {
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{Flag, Offset, Span},
    };
    let kinds = kinds(Relation::Function).to_vec();
    let mut cells = Vec::new();
    for row in 1..=2 {
        cells.extend(kinds.iter().map(|kind| match kind {
            CellKind::Node => CellValue::Node(node_id(FileId(0), Position(row))),
            CellKind::Symbol => CellValue::Symbol(symbol),
            CellKind::File => CellValue::File(FileId(0)),
            CellKind::Span => CellValue::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            CellKind::Number => CellValue::Number(Integer(0)),
            CellKind::Flag => CellValue::Flag(Flag::Lowered),
        }));
    }
    if malformed {
        let _missing = cells.pop();
    }
    Table {
        kinds,
        cells,
        count: RowCount(2),
    }
}
fn project_static_column_failure_precedes_malformed_population() {
    for malformed in [false, true] {
        assert_eq!(
            project_result(&project(99), malformed).err(),
            Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
        );
    }
    // The same malformed population must still fail when the metadata is valid.
    assert_eq!(
        project_result(&project(0), true).err(),
        Some(OperatingError::ShapeMismatch)
    );
    let valid = project_result(&project(0), false).unwrap();
    assert_eq!(valid.count.0, 2);
    assert_eq!(
        valid.cells,
        vec![
            CellValue::Node(node_id(FileId(0), Position(1))),
            CellValue::Node(node_id(FileId(0), Position(2)))
        ]
    );
}
fn node(position: u32) -> CellValue {
    CellValue::Node(node_id(FileId(17), Position(position)))
}
fn axis(direction: Direction, id: u32) -> Operation {
    Operation::Axis {
        from: PlanInput::Relation(Relation::Function),
        edges: PlanInput::Relation(Relation::Scope),
        domain: PlanInput::Relation(Relation::SyntaxTree),
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
fn collected_axis_empty_roots_do_not_acquire_unsupported_table_walk() {
    let empty = axis_result(&axis(Direction::Deep, 0), &axis_inputs(true, false)).unwrap();
    assert_eq!(empty.kinds, vec![CellKind::Node; 2]);
    assert_eq!(empty.count.0, 0);
    assert!(empty.cells.is_empty());
    // Root-id metadata is checked before the original empty-root gate.
    assert_eq!(
        axis_result(&axis(Direction::Deep, 99), &axis_inputs(true, false)).err(),
        Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    );
    // A real nonempty unsupported walk must not become an unconditional no-op.
    assert_eq!(
        axis_result(&axis(Direction::Deep, 0), &axis_inputs(false, false)).err(),
        Some(OperatingError::WrongOperation)
    );
    let child = axis_result(&axis(Direction::Child, 0), &axis_inputs(false, false)).unwrap();
    assert_eq!(child.count.0, 1);
    assert_eq!(child.cells, vec![node(1), node(2)]);
}
fn collected_axis_computed_empty_domain_keeps_its_original_shortcut() {
    let empty = axis_result(&axis(Direction::Deep, 0), &axis_inputs(false, true)).unwrap();
    assert_eq!(empty.kinds, vec![CellKind::Node; 2]);
    assert_eq!(empty.count.0, 0);
    assert_eq!(
        axis_result(&axis(Direction::Deep, 99), &axis_inputs(false, true)).err(),
        Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    );
}

/// Each subject retains its independent error-precedence and empty-result assertions.
type Input = fn();
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("static_project_column_error_precedes_malformed_population"),
        word: Word::Succeeds,
        input: || project_static_column_failure_precedes_malformed_population,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_axis_roots_do_not_acquire_unsupported_walk"),
        word: Word::Succeeds,
        input: || collected_axis_empty_roots_do_not_acquire_unsupported_table_walk,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_computed_axis_domain_keeps_shortcut_after_metadata_check"),
        word: Word::Succeeds,
        input: || collected_axis_computed_empty_domain_keeps_its_original_shortcut,
        expect: Expect::Value(()),
    },
];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
