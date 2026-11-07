//! Exact conflicting failures through the mandatory public execution owner.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::application::file_dag::program::file_session::types::SourceRows;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::service::node_id;
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
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::query_operators::types::OperationKind;
use std::cell::Cell;

type Input = fn();
type Value = ();
type Failure = OperatingError;
const SCHEMA: &[OperationKind] = &[
    OperationKind::Select,
    OperationKind::Project,
    OperationKind::Join,
    OperationKind::Union,
    OperationKind::Intersection,
    OperationKind::Difference,
];
pub(in crate::inbound::use_cases::execution::use_case_test) fn operation(kind: OperationKind, invalid: bool) -> Operation {
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
pub(in crate::inbound::use_cases::execution::use_case_test) fn population(relation: Relation, malformed: bool, symbol: Symbol) -> Table {
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
#[derive(Clone, Copy)]
pub(in crate::inbound::use_cases::execution::use_case_test) enum InputSupply {
    Present,
    MissingLeft,
    MissingRight,
    MissingAfterFirst,
    Changed,
}
pub(in crate::inbound::use_cases::execution::use_case_test) fn execute(
    plan: Plan,
    malformed: bool,
    supply: InputSupply,
) -> (Result<Table, OperatingError>, [usize; 2]) {
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("source"));
    let left = population(Relation::Function, malformed, symbol);
    let changed = population(Relation::Function, malformed, symbol);
    let right = population(Relation::Resolves, false, symbol);
    let counts = [Cell::new(0), Cell::new(0)];
    let read = |relation| match relation {
        Relation::Function => {
            counts[0].set(counts[0].get() + 1);
            if matches!(supply, InputSupply::MissingLeft)
                || (matches!(supply, InputSupply::MissingAfterFirst) && counts[0].get() > 1)
            {
                Err(OperatingError::NotRead)
            } else if matches!(supply, InputSupply::Changed) && counts[0].get() > 1 {
                Ok(SourceRows::Computed(&changed))
            } else {
                Ok(SourceRows::Computed(&left))
            }
        }
        Relation::Resolves => {
            counts[1].set(counts[1].get() + 1);
            if matches!(supply, InputSupply::MissingRight) {
                Err(OperatingError::NotRead)
            } else {
                Ok(SourceRows::Computed(&right))
            }
        }
        _ => Err(OperatingError::NotRead),
    };
    let (engine, prepared) = fixtures::prepared(plan, vec![], vec![])
        .expect("static faults stay deferred in a structurally valid request");
    let result = fixtures::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut symbols,
        &read,
    )
    .map(|(table, _)| table);
    (result, [counts[0].get(), counts[1].get()])
}
pub(in crate::inbound::use_cases::execution::use_case_test) fn layout_error(kind: OperationKind) -> OperatingError {
    match kind {
        OperationKind::Select | OperationKind::Project | OperationKind::Join => {
            OperatingError::Table(RowsError::NoSuchColumn { index: 99 })
        }
        _ => OperatingError::ShapeMismatch,
    }
}
fn schema_before_physical() {
    for &kind in SCHEMA {
        let (result, calls) = execute(
            inputs::plan(vec![operation(kind, true)]),
            true,
            InputSupply::Present,
        );
        assert_eq!(result, Err(layout_error(kind)), "{kind:?}");
        assert_eq!(
            calls,
            if matches!(kind, OperationKind::Select | OperationKind::Project) {
                [1, 0]
            } else {
                [1, 1]
            },
            "layout fails after exact availability without an extra admission reborrow: {kind:?}"
        );
    }
}
fn source_failure_before_local_layout() {
    for &kind in SCHEMA {
        let (result, calls) = execute(
            inputs::plan(vec![operation(kind, true)]),
            true,
            InputSupply::MissingLeft,
        );
        assert_eq!(result, Err(OperatingError::NotRead), "{kind:?}");
        assert_eq!(calls, [1, 0]);
    }
    for &kind in &SCHEMA[2..] {
        let (result, calls) = execute(
            inputs::plan(vec![operation(kind, true)]),
            true,
            InputSupply::MissingRight,
        );
        assert_eq!(
            result,
            Err(OperatingError::NotRead),
            "the declared later source remains authoritative: {kind:?}"
        );
        assert_eq!(calls, [1, 1]);
    }
}
fn valid_populations_and_generation_binding() {
    for &kind in SCHEMA {
        let (result, _) = execute(
            inputs::plan(vec![operation(kind, false)]),
            false,
            InputSupply::Present,
        );
        let result = result.expect("valid metadata and physical populations execute normally");
        assert_eq!(
            result.count.0,
            match kind {
                OperationKind::Join => 4,
                OperationKind::Difference => 0,
                _ => 2,
            },
            "{kind:?}"
        );
        if matches!(kind, OperationKind::Select | OperationKind::Project) {
            assert_eq!(
                result.cells,
                [1, 2].map(|row| CellValue::Node(node_id(FileId(0), Position(row))))
            );
        }
    }
    let (changed, calls) = execute(
        inputs::plan(vec![operation(OperationKind::Project, false)]),
        false,
        InputSupply::Changed,
    );
    assert_eq!(
        changed,
        Err(OperatingError::NotRead),
        "a matching schema does not replace the available source's physical identity"
    );
    assert_eq!(calls, [2, 0]);
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_six_schema_first_layout_faults_precede_malformed_physical_populations"),
        word: Word::Succeeds,
        input: || schema_before_physical,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("actual_source_availability_faults_precede_local_static_layout"),
        word: Word::Succeeds,
        input: || source_failure_before_local_layout,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("valid_populations_and_exact_snapshot_identity_are_preserved"),
        word: Word::Succeeds,
        input: || valid_populations_and_generation_binding,
        expect: Expect::Value(()),
    },];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
