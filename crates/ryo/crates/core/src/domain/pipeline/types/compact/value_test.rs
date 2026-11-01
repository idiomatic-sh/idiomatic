//! Real compact Feed/hold, join, frontier and provider-refusal differential proof.

use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, Operand, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};

/// This proof supplies its own complete resident file fixture.
type Input = ();
/// Completion means every transport, output and refusal assertion held.
type Value = ();
/// Ordinary pipeline/provider refusals.
type Failure = crate::domain::operating::errors::OperatingError;

/// One end-to-end case exercises both plans in the same physical generation.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("compact_chains_cross_rule_join_and_frontier_match_uncached_execution"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

/// Real Feed/hold and exported/frontier boundaries, with two distinct plans sharing one generation.
fn subject(_input: Input) -> Result<Value, Failure> {
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::service::pool_for;
    use crate::domain::pipeline::service::evaluate_file;
    use crate::domain::stored_indexes::service::{LIMIT, new};
    use crate::domain::stored_selection::service::{activate, prepare};
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
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    };
    let join = PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Join {
            left: Source::Step(StepId(1)),
            right: Source::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
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
    prepare(&mut compact, &plans, &[Vec::new(), Vec::new()]);
    let mut cache = new(&[Relation::Function], &[], LIMIT).unwrap();
    activate(&mut cache, &compact, &[Relation::Function], |_| true);
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    for plan in &plans {
        evaluate_file(plan, &read, None, &mut context, &mut baseline)?;
        {
            let mut actual = OperatorContext {
                stored_indexes: Some(&mut cache),
                bindings: context.bindings,
                symbols: &mut *context.symbols,
                source: context.source,
                paths: context.paths,
                arenas: context.arenas,
                memo: &mut *context.memo,
                scratch: &mut *context.scratch,
            };
            evaluate_file(plan, &read, None, &mut actual, &mut compact)?;
        }
        assert_eq!(compact.steps[2], baseline.steps[2]);
        assert_eq!(compact.frontier, baseline.frontier);
        assert_eq!(compact.steps[2].count.0, 8);
        assert!(compact.selected[0].is_some());
        assert!(compact.steps[0].cells.is_empty());
        assert!(
            compact
                .rooms
                .iter()
                .flatten()
                .all(|room| room.held.iter().all(|table| table.cells.is_empty()))
        );
        crate::domain::working_set::service::clear_rule(&mut compact);
        assert!(compact.selected.iter().all(Option::is_none));
        assert_eq!(compact.frontier[0].count.0, 8);
    }
    assert_eq!(cache.selection_counts.builds, 1);
    assert!(cache.selection_counts.reuses >= 3);
    readable.set(false);
    let refused = {
        let mut actual = OperatorContext {
            stored_indexes: Some(&mut cache),
            bindings: context.bindings,
            symbols: &mut *context.symbols,
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        evaluate_file(&plans[0], &read, None, &mut actual, &mut compact)
    };
    assert!(matches!(
        refused,
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
    assert!(compact.selected.iter().all(Option::is_none));
    assert!(matches!(
        evaluate_file(&plans[0], &read, None, &mut context, &mut baseline),
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
    Ok(())
}

/// Execute the compact transport differential case through the registered test entrypoint.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
