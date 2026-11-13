//! Exact conflicting failures through the mandatory public execution owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::service::node_position;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Span;
use ryo_relations::domain::relations::types::Value as CellValue;
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "this constructor names exactly the six declared schema-first kinds; any other kind is a fixture bug that must crash"
)]
pub(crate) fn operation(kind: OperationKind, invalid: bool) -> Operation {
    let left = PlanInput::Relation(Relation::Function);
    let right = PlanInput::Relation(if invalid || kind == OperationKind::Join {
        Relation::Resolves
    } else {
        Relation::Function
    });
    let column = ColumnIndex(if invalid { 99 } else { 0 });
    match kind {
        OperationKind::Select => Operation::Select {
            from: left,
            comparisons: vec![],
            kept: vec![column],
        },
        OperationKind::Project => Operation::Project {
            from: left,
            columns: vec![column],
            distinct: RepeatedRows::Kept,
        },
        OperationKind::Join => Operation::Join {
            left,
            right,
            left_on: vec![],
            right_on: vec![],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column,
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        OperationKind::Union => Operation::Union { left, right },
        OperationKind::Intersection => Operation::Intersection { left, right },
        OperationKind::Difference => Operation::Difference { left, right },
        _ => panic!("this constructor names exactly the six declared schema-first kinds"),
    }
}
pub(crate) fn population(relation: Relation, malformed: bool, symbol: Symbol) -> Table {
    let kinds = kinds(relation).to_vec();
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
        let _truncated = cells.pop();
    }
    Table {
        kinds,
        cells,
        count: RowCount(2),
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum InputSupply {
    Present,
    MissingLeft,
    MissingRight,
    Changed,
}
pub(crate) fn execute(
    plan: Plan,
    malformed: bool,
    supply: InputSupply,
) -> (Result<Table, OperatingError>, [usize; 2]) {
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("source"));
    let left = population(Relation::Function, malformed, symbol);
    let changed = population(Relation::Function, malformed, symbol);
    let right = population(Relation::Resolves, false, symbol);
    let (left, changed, right) = (
        fixtures::facts::columns_of(&left),
        fixtures::facts::columns_of(&changed),
        fixtures::facts::columns_of(&right),
    );
    let counts = [AtomicUsize::new(0), AtomicUsize::new(0)];
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the fake serves the join's two sides; every other relation is not read"
    )]
    let read = |relation| match relation {
        Relation::Function => {
            let reads = counts[0].fetch_add(1, Ordering::Relaxed) + 1;
            if matches!(supply, InputSupply::MissingLeft) {
                Err(fixtures::facts::not_held())
            } else if matches!(supply, InputSupply::Changed) && reads > 1 {
                Ok(fixtures::facts::view(relation, &changed))
            } else {
                Ok(fixtures::facts::view(relation, &left))
            }
        }
        Relation::Resolves => {
            let _reads = counts[1].fetch_add(1, Ordering::Relaxed);
            if matches!(supply, InputSupply::MissingRight) {
                Err(fixtures::facts::not_held())
            } else {
                Ok(fixtures::facts::view(relation, &right))
            }
        }
        _ => Err(fixtures::facts::not_held()),
    };
    let facts = fixtures::facts::ReadFacts::new(read);
    let (engine, prepared) = fixtures::requests::prepared(plan, vec![], vec![])
        .expect("static faults stay deferred in a structurally valid request");
    let result = fixtures::requests::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut symbols,
        &facts,
    );
    let [left, right] = counts.map(AtomicUsize::into_inner);
    (result, [left, right])
}
/// What an admitted plan answered: its row count, the positions of its node cells in output order,
/// and how often each side was read.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    pub(crate) rows: usize,
    pub(crate) nodes: Cow<'static, [u32]>,
    pub(crate) reads: [usize; 2],
}
/// What a refused plan answered: the error and how often each side was read before it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Refusal {
    pub(crate) error: OperatingError,
    pub(crate) reads: [usize; 2],
}
pub(crate) fn observed(
    result: Result<Table, OperatingError>,
    reads: [usize; 2],
) -> Result<Answer, Refusal> {
    result
        .map(|table| Answer {
            rows: table.count.0,
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
            reads,
        })
        .map_err(|error| Refusal { error, reads })
}
/// One schema-first operation over the join's two sides.
#[derive(Clone, Copy, Debug)]
struct Admission {
    kind: OperationKind,
    invalid: bool,
    supply: InputSupply,
}
type Input = Admission;
type Value = Answer;
type Failure = Refusal;
const fn admission(kind: OperationKind, invalid: bool, supply: InputSupply) -> Admission {
    Admission {
        kind,
        invalid,
        supply,
    }
}
const fn refused(error: OperatingError, reads: [usize; 2]) -> Expect<Value, Failure> {
    Expect::Error(Refusal { error, reads })
}
const fn answered(rows: usize, nodes: &'static [u32], reads: [usize; 2]) -> Expect<Value, Failure> {
    Expect::Value(Answer {
        rows,
        nodes: Cow::Borrowed(nodes),
        reads,
    })
}
const NO_COLUMN: OperatingError = OperatingError::Table(RowsError::NoSuchColumn { index: 99 });
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_six_schema_first_layout_faults_precede_malformed_physical_populations"),
        word: Word::Refuses,
        input: || admission(OperationKind::Select, true, InputSupply::Present),
        expect: refused(NO_COLUMN, [1, 0]),
    },
    Case {
        name: CaseName("a_projection_layout_fault_precedes_its_malformed_population"),
        word: Word::Refuses,
        input: || admission(OperationKind::Project, true, InputSupply::Present),
        expect: refused(NO_COLUMN, [1, 0]),
    },
    Case {
        name: CaseName("a_join_layout_fault_precedes_its_malformed_populations"),
        word: Word::Refuses,
        input: || admission(OperationKind::Join, true, InputSupply::Present),
        expect: refused(NO_COLUMN, [1, 1]),
    },
    Case {
        name: CaseName("a_union_shape_fault_precedes_its_malformed_populations"),
        word: Word::Refuses,
        input: || admission(OperationKind::Union, true, InputSupply::Present),
        expect: refused(OperatingError::ShapeMismatch, [1, 1]),
    },
    Case {
        name: CaseName("an_intersection_shape_fault_precedes_its_malformed_populations"),
        word: Word::Refuses,
        input: || admission(OperationKind::Intersection, true, InputSupply::Present),
        expect: refused(OperatingError::ShapeMismatch, [1, 1]),
    },
    Case {
        name: CaseName("a_difference_shape_fault_precedes_its_malformed_populations"),
        word: Word::Refuses,
        input: || admission(OperationKind::Difference, true, InputSupply::Present),
        expect: refused(OperatingError::ShapeMismatch, [1, 1]),
    },
    Case {
        name: CaseName("actual_source_availability_faults_precede_local_static_layout"),
        word: Word::Refuses,
        input: || admission(OperationKind::Select, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_missing_left_source_precedes_a_projection_layout_fault"),
        word: Word::Refuses,
        input: || admission(OperationKind::Project, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_missing_left_source_precedes_a_join_layout_fault"),
        word: Word::Refuses,
        input: || admission(OperationKind::Join, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_missing_left_source_precedes_a_union_shape_fault"),
        word: Word::Refuses,
        input: || admission(OperationKind::Union, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_missing_left_source_precedes_an_intersection_shape_fault"),
        word: Word::Refuses,
        input: || admission(OperationKind::Intersection, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_missing_left_source_precedes_a_difference_shape_fault"),
        word: Word::Refuses,
        input: || admission(OperationKind::Difference, true, InputSupply::MissingLeft),
        expect: refused(OperatingError::NotRead, [1, 0]),
    },
    Case {
        name: CaseName("a_declared_later_join_source_remains_authoritative"),
        word: Word::Refuses,
        input: || admission(OperationKind::Join, true, InputSupply::MissingRight),
        expect: refused(OperatingError::NotRead, [1, 1]),
    },
    Case {
        name: CaseName("a_declared_later_union_source_remains_authoritative"),
        word: Word::Refuses,
        input: || admission(OperationKind::Union, true, InputSupply::MissingRight),
        expect: refused(OperatingError::NotRead, [1, 1]),
    },
    Case {
        name: CaseName("a_declared_later_intersection_source_remains_authoritative"),
        word: Word::Refuses,
        input: || admission(OperationKind::Intersection, true, InputSupply::MissingRight),
        expect: refused(OperatingError::NotRead, [1, 1]),
    },
    Case {
        name: CaseName("a_declared_later_difference_source_remains_authoritative"),
        word: Word::Refuses,
        input: || admission(OperationKind::Difference, true, InputSupply::MissingRight),
        expect: refused(OperatingError::NotRead, [1, 1]),
    },
    Case {
        name: CaseName("valid_populations_and_exact_snapshot_identity_are_preserved"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Select, false, InputSupply::Present),
        expect: answered(2, &[1, 2], [4, 0]),
    },
    Case {
        name: CaseName("a_valid_projection_keeps_both_node_rows"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Project, false, InputSupply::Present),
        expect: answered(2, &[1, 2], [4, 0]),
    },
    Case {
        name: CaseName("a_valid_join_pairs_every_row"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Join, false, InputSupply::Present),
        expect: answered(4, &[1, 1, 1, 2, 2, 1, 2, 2], [3, 3]),
    },
    Case {
        name: CaseName("a_valid_union_keeps_both_rows"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Union, false, InputSupply::Present),
        expect: answered(2, &[1, 2], [4, 0]),
    },
    Case {
        name: CaseName("a_valid_intersection_keeps_both_rows"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Intersection, false, InputSupply::Present),
        expect: answered(2, &[1, 2], [4, 0]),
    },
    Case {
        name: CaseName("a_valid_difference_of_a_source_with_itself_is_empty"),
        word: Word::Succeeds,
        input: || admission(OperationKind::Difference, false, InputSupply::Present),
        expect: answered(0, &[], [4, 0]),
    },
    Case {
        name: CaseName("a_matching_schema_does_not_replace_the_sources_physical_identity"),
        word: Word::Refuses,
        input: || admission(OperationKind::Project, false, InputSupply::Changed),
        expect: refused(OperatingError::NotRead, [2, 0]),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let (result, reads) = execute(
        inputs::plan(vec![operation(input.kind, input.invalid)]),
        input.invalid,
        input.supply,
    );
    observed(result, reads)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
