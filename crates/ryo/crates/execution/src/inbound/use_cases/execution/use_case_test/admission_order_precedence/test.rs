//! Preflight precedence of producer failures, stored prefixes and registered kinds through the mandatory public execution owner.
use super::super::super::fixtures;
use super::super::admission_order::test::{
    InputSupply, execute, layout_error, operation, population,
};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::operator::contract::service::semantics;
use ryo_execution_domain::domain::operator::contract::types::PreflightOrder;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
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
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

type Input = fn();
type Value = ();
type Failure = OperatingError;
fn child_failure_and_resources_first() {
    let first = operation(OperationKind::Project, false);
    let last = Operation::Project {
        from: PlanInput::Step(StepId(0)),
        columns: vec![ColumnIndex(99)],
        distinct: RepeatedRows::Kept,
    };
    let (result, _) = execute(inputs::plan(vec![first, last]), true, InputSupply::Present);
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
    let (result, calls) = execute(
        inputs::plan(vec![count]),
        true,
        InputSupply::MissingAfterFirst,
    );
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
    use ryo_relations::domain::relations::service::to_bits;
    use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
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
                kinds: kinds(Relation::Function),
                columns: &columns,
            }))
        };
        let (engine, prepared) = fixtures::requests::prepared(
            inputs::plan(vec![operation(OperationKind::Project, bad_layout)]),
            vec![],
            vec![],
        )
        .unwrap();
        let result = fixtures::requests::run_prepared(
            &engine,
            &prepared,
            b"",
            &Elements::default(),
            &mut symbols,
            &read,
        );
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
        assert_eq!(semantics(kind).contract.preflight, expected, "{kind:?}");
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
