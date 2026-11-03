//! Real compact Feed/hold, join, frontier and provider-refusal differential proof.
use crate::application::file_dag::feed::Feed;

use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, Operand, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};

/// This proof supplies its own complete resident file fixture.
type Input = bool;
/// Completion means every transport, output and refusal assertion held.
type Value = ();
/// Ordinary pipeline/provider refusals.
type Failure = crate::domain::operating::errors::OperatingError;

/// One end-to-end case exercises both plans in the same physical generation.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("compact_chains_cross_rule_join_and_frontier_match_uncached_execution"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("compact_distinct_join_frontier_and_provider_refusal_match_fallback"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(()),
    },
];

/// Real Feed/hold and exported/frontier boundaries, with two distinct plans sharing one generation.
fn subject(distinct: Input) -> Result<Value, Failure> {
    subject_with_counting(
        distinct,
        crate::domain::lowering::types::Counting::DoesNotCount,
    )
}

/// Exercise the same real selected-input fixture with explicit work accounting.
fn subject_with_counting(
    distinct: Input,
    counting: crate::domain::lowering::types::Counting,
) -> Result<Value, Failure> {
    use crate::application::file_dag::derived::selection::prepare_request;
    use crate::application::file_dag::pipeline::service::{
        evaluate_file, evaluate_file_with_executor,
    };
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::service::pool_for;
    use crate::domain::table_scratch::types::OperatorScratch;
    use crate::domain::table_texts::types::SourceView;
    use ryo_language::domain::relations::service::{node_id, to_bits};
    use ryo_language::domain::relations::types::{
        CellKind, ColumnsView, FactColumn, FileId, Offset, Position, Relation, Span, Value as Cell,
    };
    use ryo_language::domain::symbols::service::intern_derived;
    use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
    use ryo_language::domain::terms::types::NameText;
    use ryo_values::domain::plan_parts::types::{JoinColumn, LiteralText, QueryLiteral, Side};
    use ryo_values::domain::plans::types::{Emit, Plan};
    let mut symbols = Symbols::default();
    let keep = intern_derived(&mut symbols, ByteSlice(b""), NameText("keep"));
    let skip = intern_derived(&mut symbols, ByteSlice(b""), NameText("skip"));
    let columns = [
        FactColumn(
            (0..64)
                .map(|row| to_bits(Cell::Node(node_id(FileId(0), Position(row)))).0)
                .collect(),
        ),
        FactColumn(
            (0..64)
                .map(|row| to_bits(Cell::Symbol(if row % 8 == 0 { keep } else { skip })).0)
                .collect(),
        ),
        FactColumn(vec![to_bits(Cell::File(FileId(0))).0; 64]),
        FactColumn(
            (0..64)
                .map(|row| {
                    to_bits(Cell::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }))
                    .0
                })
                .collect(),
        ),
    ];
    let schema: &[CellKind] = ryo_language::domain::relations::service::kinds(Relation::Function);
    let source = ColumnsView {
        kinds: schema,
        columns: &columns,
    };
    let filter = PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Source::Relation(Relation::Function),
            kept: vec![ColumnIndex(0), ColumnIndex(2)],
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("keep".into()))),
                origin: None,
            }],
        },
    };
    let project = PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: Source::Step(StepId(0)),
            columns: vec![ColumnIndex(u32::from(distinct))],
            distinct: if distinct {
                RepeatedRows::Dropped
            } else {
                RepeatedRows::Kept
            },
        },
    };
    let join = PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Join {
            left: Source::Step(StepId(1)),
            right: Source::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(u32::from(distinct))],
            filter: Vec::new(),
            view: Vec::new(),
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],

            reach: None,
        },
    };
    let first = Plan {
        parameters: Vec::new(),
        steps: vec![filter, project, join],
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(1),
            key: vec![ColumnIndex(0)],
            headings: Vec::new(),
        },
        frontier: vec![Source::Step(StepId(0))],
    };
    let plans = [first.clone(), first];
    let readable = std::cell::Cell::new(true);
    let read = |relation| (relation == Relation::Function && readable.get()).then_some(source);
    let mut baseline = pool_for(&plans);
    let mut compact = pool_for(&plans);
    let mut baseline_prepared = crate::domain::operator::types::PreparedPool::default();
    let mut prepared = crate::domain::operator::types::PreparedPool::default();
    baseline_prepared.counting = counting;
    prepared.counting = counting;
    let request = crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
        &plans,
        &[vec![], vec![]],
        &[],
        &|relation| {
            read(relation).map_or_else(
                || ryo_language::domain::relations::service::kinds(relation),
                |view| view.kinds,
            )
        },
    )?;
    prepare_request(&mut prepared, &request);
    assert_eq!(
        prepared.selection_plans[0].steps[1]
            .as_ref()
            .unwrap()
            .distinct,
        distinct,
    );
    assert!(prepared.selection_plans[0].steps[2].is_none());
    let frame = std::cell::RefCell::new(
        crate::application::file_dag::derived::owner::resource_fixture::residual_frame(
            &plans,
            &prepared,
            &[Relation::Function],
            &[],
        ),
    );
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    for (rule, plan) in plans.iter().enumerate() {
        evaluate_file(
            plan,
            &read,
            None,
            &mut context,
            &mut baseline,
            &mut baseline_prepared,
            &[],
        )?;
        {
            let mut actual = OperatorContext {
                prepared_kinds: None,
                prepared_matchers: None,
                bindings: context.bindings,
                symbols: ryo_language::domain::symbols::service::query_reborrow(
                    &mut context.symbols,
                ),
                source: context.source,
                paths: context.paths,
                arenas: context.arenas,
                memo: &mut *context.memo,
                scratch: &mut *context.scratch,
            };
            evaluate_file_with_executor(
                plan,
                &read,
                None,
                &mut actual,
                &mut compact,
                &mut prepared,
                &[],
                None,
                rule,
                Some(&frame),
                &crate::application::file_dag::execution::CommonOperatorEngine,
                &request.rules[rule],
            )?;
        }
        assert_eq!(compact.steps[2], baseline.steps[2]);
        assert_eq!(compact.frontier, baseline.frontier);
        assert_eq!(compact.steps[2].count.0, 8);
        if counting == crate::domain::lowering::types::Counting::Counts {
            assert!(
                compact.selected[0].is_none(),
                "counted work uses the ordinary materialized input path"
            );
            assert_eq!(
                prepared.fed, baseline_prepared.fed,
                "counted fallback preserves the ordinary input work"
            );
            assert!(
                prepared
                    .fed
                    .iter()
                    .flatten()
                    .any(|rows| rows.0.iter().any(|count| *count > 0)),
                "the counted path must record actual fed rows"
            );
        } else {
            assert!(compact.selected[0].is_some());
            assert!(compact.steps[0].cells.is_empty());
            assert!(compact.steps[1].cells.is_empty());
            assert_eq!(
                frame
                    .borrow()
                    .relations
                    .derived
                    .retained_predicate_rows()
                    .expect("owner retains predicate IDs"),
                8,
                "shared IDs remain predicate-only"
            );
            assert!(
                compact
                    .rooms
                    .iter()
                    .flatten()
                    .all(|room| room.held.iter().all(|table| table.cells.is_empty()))
            );
        }
        let frontier = std::mem::take(&mut compact.frontier);
        drop(compact);
        compact = pool_for(&plans);
        assert!(compact.selected.is_empty());
        assert_eq!(
            frontier[0].count.0, 8,
            "application-owned output outlives its rule workspace"
        );
    }
    if counting == crate::domain::lowering::types::Counting::DoesNotCount {
        assert_eq!(frame.borrow().relations.derived.selection_counts().0, 1);
        assert!(frame.borrow().relations.derived.selection_counts().1 >= 3);
    }
    readable.set(false);
    let refused = {
        let mut actual = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: context.bindings,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        evaluate_file_with_executor(
            &plans[0],
            &read,
            None,
            &mut actual,
            &mut compact,
            &mut prepared,
            &[],
            None,
            0,
            Some(&frame),
            &crate::application::file_dag::execution::CommonOperatorEngine,
            &request.rules[0],
        )
    };
    assert!(matches!(
        refused,
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
    assert!(compact.selected.iter().all(Option::is_none));
    assert!(matches!(
        evaluate_file(
            &plans[0],
            &read,
            None,
            &mut context,
            &mut baseline,
            &mut baseline_prepared,
            &[]
        ),
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
    Ok(())
}

/// Execute the compact transport differential case through the registered test entrypoint.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[path = "counting_test.rs"]
mod counting_test;
