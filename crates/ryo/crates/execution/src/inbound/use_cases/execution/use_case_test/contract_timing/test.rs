//! Conflicting metadata, population and resource failures through the actual execution owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
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
use ryo_relations::domain::relations::service::node_position;
use std::borrow::Cow;

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
        fixtures::requests::prepared(inputs::plan(vec![operation.clone()]), vec![], vec![])?;
    fixtures::requests::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut symbols,
        &read,
    )
}
/// Canonical facts and declared projections reach the same mandatory owner, not a primitive harness.
fn axis_result(operation: &Operation, tables: &[Table]) -> Result<Table, OperatingError> {
    let references = tables.iter().collect::<Vec<_>>();
    let mut symbols = Symbols::default();
    fixtures::requests::table_request(operation.clone(), &references, vec![], &mut symbols)
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
/// One operation through the owner: a projection of a column over the function population (malformed or not), or an
/// axis over roots, edges and a domain (either of the first and last empty or not).
#[derive(Clone, Copy, Debug)]
enum Timing {
    Project {
        column: u32,
        malformed: bool,
    },
    Axis {
        direction: Direction,
        id: u32,
        empty_roots: bool,
        empty_domain: bool,
    },
}
/// What a completed operation answered: its schema, row count and the positions of its node cells in order.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    kinds: Cow<'static, [CellKind]>,
    rows: usize,
    cells: usize,
    nodes: Cow<'static, [u32]>,
}
type Input = Timing;
type Value = Answer;
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let table = match input {
        Timing::Project { column, malformed } => project_result(&project(column), malformed),
        Timing::Axis {
            direction,
            id,
            empty_roots,
            empty_domain,
        } => axis_result(
            &axis(direction, id),
            &axis_inputs(empty_roots, empty_domain),
        ),
    }?;
    Ok(Answer {
        rows: table.count.0,
        cells: table.cells.len(),
        nodes: table
            .cells
            .iter()
            .filter_map(|cell| {
                if let CellValue::Node(node) = cell {
                    Some(node_position(*node).0)
                } else {
                    None
                }
            })
            .collect(),
        kinds: table.kinds.into(),
    })
}
const fn projected(column: u32, malformed: bool) -> Timing {
    Timing::Project { column, malformed }
}
const fn walked(direction: Direction, id: u32, empty_roots: bool, empty_domain: bool) -> Timing {
    Timing::Axis {
        direction,
        id,
        empty_roots,
        empty_domain,
    }
}
const fn answer(
    kinds: &'static [CellKind],
    rows: usize,
    nodes: &'static [u32],
) -> Expect<Value, Failure> {
    Expect::Value(Answer {
        kinds: Cow::Borrowed(kinds),
        rows,
        cells: nodes.len(),
        nodes: Cow::Borrowed(nodes),
    })
}
const NO_COLUMN: Expect<Value, Failure> =
    Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }));
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("static_project_column_error_precedes_malformed_population"),
        word: Word::Refuses,
        input: || projected(99, false),
        expect: NO_COLUMN,
    },
    Case {
        name: CaseName("a_static_project_column_error_precedes_a_malformed_population"),
        word: Word::Refuses,
        input: || projected(99, true),
        expect: NO_COLUMN,
    },
    Case {
        name: CaseName("a_malformed_population_still_fails_under_valid_metadata"),
        word: Word::Refuses,
        input: || projected(0, true),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("a_valid_projection_answers_both_function_nodes"),
        word: Word::Succeeds,
        input: || projected(0, false),
        expect: answer(&[CellKind::Node], 2, &[1, 2]),
    },
    Case {
        name: CaseName("empty_axis_roots_do_not_acquire_unsupported_walk"),
        word: Word::Succeeds,
        input: || walked(Direction::Deep, 0, true, false),
        expect: answer(&[CellKind::Node, CellKind::Node], 0, &[]),
    },
    Case {
        name: CaseName("root_id_metadata_is_checked_before_the_empty_root_gate"),
        word: Word::Refuses,
        input: || walked(Direction::Deep, 99, true, false),
        expect: NO_COLUMN,
    },
    Case {
        name: CaseName("a_nonempty_unsupported_walk_is_not_a_no_op"),
        word: Word::Refuses,
        input: || walked(Direction::Deep, 0, false, false),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_supported_child_walk_answers_its_one_edge"),
        word: Word::Succeeds,
        input: || walked(Direction::Child, 0, false, false),
        expect: answer(&[CellKind::Node, CellKind::Node], 1, &[1, 2]),
    },
    Case {
        name: CaseName("empty_computed_axis_domain_keeps_shortcut_after_metadata_check"),
        word: Word::Succeeds,
        input: || walked(Direction::Deep, 0, false, true),
        expect: answer(&[CellKind::Node, CellKind::Node], 0, &[]),
    },
    Case {
        name: CaseName("an_empty_domain_still_refuses_a_missing_root_id"),
        word: Word::Refuses,
        input: || walked(Direction::Deep, 99, false, true),
        expect: NO_COLUMN,
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
