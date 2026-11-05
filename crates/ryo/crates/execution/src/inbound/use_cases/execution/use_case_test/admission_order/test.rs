//! Exact conflicting failures through the mandatory public execution owner.
use super::super::super::{
    fixtures,
    types::{OperatingError, SourceRows},
};
use crate::domain::operator::contract::{service::semantics, types::PreflightOrder};
use ryo_language::domain::{
    elements::types::Elements,
    symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    },
};
use ryo_relations::domain::{
    relations::{
        service::{kinds, node_id},
        types::{
            CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as CellValue,
        },
    },
    terms::types::{NameText, Symbol},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, Input as PlanInput, JoinColumn, RepeatedRows, Side, StepId,
        },
        plans::types::{Operation, OperationKind, Plan},
        rows::{
            errors::TableModelError,
            types::{RowCount, Table},
        },
    },
};
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

fn operation(kind: OperationKind, invalid: bool) -> Operation {
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
fn population(relation: Relation, malformed: bool, symbol: Symbol) -> Table {
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
enum Supply {
    Present,
    MissingLeft,
    MissingRight,
    MissingAfterFirst,
    Changed,
}
fn execute(
    plan: Plan,
    malformed: bool,
    supply: Supply,
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
            if matches!(supply, Supply::MissingLeft)
                || (matches!(supply, Supply::MissingAfterFirst) && counts[0].get() > 1)
            {
                Err(OperatingError::NotRead)
            } else if matches!(supply, Supply::Changed) && counts[0].get() > 1 {
                Ok(SourceRows::Computed(&changed))
            } else {
                Ok(SourceRows::Computed(&left))
            }
        }
        Relation::Resolves => {
            counts[1].set(counts[1].get() + 1);
            if matches!(supply, Supply::MissingRight) {
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
fn layout_error(kind: OperationKind) -> OperatingError {
    match kind {
        OperationKind::Select | OperationKind::Project | OperationKind::Join => {
            OperatingError::Table(TableModelError::NoSuchColumn { index: 99 })
        }
        _ => OperatingError::ShapeMismatch,
    }
}
fn schema_before_physical() {
    for &kind in SCHEMA {
        let (result, calls) = execute(
            fixtures::plan(vec![operation(kind, true)]),
            true,
            Supply::Present,
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
            fixtures::plan(vec![operation(kind, true)]),
            true,
            Supply::MissingLeft,
        );
        assert_eq!(result, Err(OperatingError::NotRead), "{kind:?}");
        assert_eq!(calls, [1, 0]);
    }
    for &kind in &SCHEMA[2..] {
        let (result, calls) = execute(
            fixtures::plan(vec![operation(kind, true)]),
            true,
            Supply::MissingRight,
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
            fixtures::plan(vec![operation(kind, false)]),
            false,
            Supply::Present,
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
        fixtures::plan(vec![operation(OperationKind::Project, false)]),
        false,
        Supply::Changed,
    );
    assert_eq!(
        changed,
        Err(OperatingError::NotRead),
        "a matching schema does not replace the available source's physical identity"
    );
    assert_eq!(calls, [2, 0]);
}
fn child_failure_and_resources_first() {
    let first = operation(OperationKind::Project, false);
    let last = Operation::Project {
        from: PlanInput::Step(StepId(0)),
        columns: vec![ColumnIndex(99)],
        distinct: RepeatedRows::Kept,
    };
    let (result, _) = execute(fixtures::plan(vec![first, last]), true, Supply::Present);
    assert_eq!(
        result,
        Err(OperatingError::ShapeMismatch),
        "the actual child fails before its consumer's local metadata"
    );
    let count = Operation::GroupCount {
        from: PlanInput::Relation(Relation::Function),
        keys: vec![ColumnIndex(99)],
        distinct: None,
    };
    let (result, calls) = execute(fixtures::plan(vec![count]), true, Supply::MissingAfterFirst);
    assert_eq!(
        result,
        Err(OperatingError::ShapeMismatch),
        "resource-first metadata is not hoisted before physical admission"
    );
    assert_eq!(
        calls,
        [1, 0],
        "resource-first admission inspects the first malformed borrow; the later NotRead population is never requested"
    );
}
fn stored_prefix_keeps_static_error_order_and_source_identity() {
    use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
    use ryo_values::domain::table_words::service::to_bits;
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("prefix"));
    let table = population(Relation::Function, false, symbol);
    let mut columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                (0..table.count.0)
                    .map(|row| to_bits(table.cells[row * table.kinds.len() + column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let _tail = columns.last_mut().unwrap().0.pop();
    for bad_layout in [false, true] {
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            Ok(SourceRows::Stored(ColumnsView {
                kinds: &table.kinds,
                columns: &columns,
            }))
        };
        let (engine, prepared) = fixtures::prepared(
            fixtures::plan(vec![operation(OperationKind::Project, bad_layout)]),
            vec![],
            vec![],
        )
        .unwrap();
        let result = fixtures::run_prepared(
            &engine,
            &prepared,
            b"",
            &Elements::default(),
            &mut symbols,
            &read,
        )
        .map(|(table, _)| table);
        if bad_layout {
            assert_eq!(result, Err(layout_error(OperationKind::Project)));
            assert_eq!(reads.get(), 1, "static refusal commits no provider reread");
        } else {
            let result =
                result.expect("readable ragged storage normalizes the first complete prefix");
            assert_eq!(result.count.0, 1);
            assert_eq!(
                result.cells,
                [CellValue::Node(node_id(FileId(0), Position(1)))]
            );
        }
    }
}
fn exhaustive_contract_declarations() {
    use OperationKind::*;
    assert_eq!(OperationKind::ALL.len(), 26);
    for &kind in OperationKind::ALL {
        let expected = match kind {
            Select | Project | Join | Union | Intersection | Difference => {
                PreflightOrder::SchemaFirst
            }
            Extend | Antijoin | GroupCount | GroupDigest | Capture | Words | Replace | Rank
            | SpanBetween | SpanBounds | OrderedMatch | Axis | Closure | Cycles | OrderedDfs
            | Rows | Hosted | SourceMatches | RenderSyntax | FitsRendered => {
                PreflightOrder::ResourcesFirst
            }
        };
        assert_eq!(semantics(kind).0.preflight, expected, "{kind:?}");
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("readable_stored_prefix_preserves_schema_precedence_and_actual_rows"),
        word: Word::Succeeds,
        input: || stored_prefix_keeps_static_error_order_and_source_identity,
        expect: Expect::Value(()),
    },
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
    },
    Case {
        name: CaseName("producer_failures_and_resource_first_physical_order_are_not_overridden"),
        word: Word::Succeeds,
        input: || child_failure_and_resources_first,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("every_registered_kind_has_an_explicit_independent_preflight_policy"),
        word: Word::Succeeds,
        input: || exhaustive_contract_declarations,
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
