//! The replay of a selection class result: the accepted prefix first, then the terminal error.

use crate::domain::operator::types::{Emit, OperatorContext};
use crate::domain::{
    operating::errors::OperatingError,
    replay::{service::selected, types::SelectionColumns},
    shared_walk::{
        service_test::{evaluate_rows, relation_plan, with_context},
        types::{WalkWork, prepared_walk::value::PreparedWalk},
    },
    step_classes::types::ClassPlace,
    table_model::types::Rows,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::{
    relations::types::{FileId, NodeId, Offset, Relation, Span, Value as CellValue},
    terms::types::NameText,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    rows::types::RowCount,
};

/// What a row of the cases replays.
#[derive(Clone, Copy)]
enum Scenario {
    /// The accepted prefix of a result whose terminal predicate failed, projected to columns.
    PrefixBeforeTheError,
    /// The same result, asked for the error that follows the prefix.
    TheTerminalError,
    /// The same result into a sink that refuses the first row.
    ASinkThatRefuses,
}

/// What a row of the cases asks.
type Input = Scenario;
/// How many rows the sink was handed.
type Value = usize;
/// What the replay refuses with.
type Failure = OperatingError;

#[derive(Default)]
struct Collected(Vec<Vec<CellValue>>);
impl Emit for Collected {
    fn row(
        &mut self,
        _: &mut OperatorContext<'_>,
        row: &[CellValue],
    ) -> Result<(), OperatingError> {
        self.0.push(row.to_vec());
        Ok(())
    }
}

struct Refuse;
impl Emit for Refuse {
    fn row(&mut self, _: &mut OperatorContext<'_>, _: &[CellValue]) -> Result<(), OperatingError> {
        Err(OperatingError::NotRead)
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let comparison = PlanComparison {
        test: Test::Matches,
        left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        right: Operand::Column(ColumnIndex(2)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let plans = [relation_plan(Relation::SemanticSyntax, vec![comparison])];
    let prepared = PreparedWalk::compile(&plans, &[vec![]]);
    let mut symbols = Symbols::default();
    let valid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("x"),
    );
    let invalid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("["),
    );
    let table = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SemanticSyntax).to_vec(),
        cells: [valid, invalid, valid]
            .iter()
            .enumerate()
            .flat_map(|(at, text)| {
                [
                    CellValue::Node(NodeId(at as u64)),
                    CellValue::Symbol(valid),
                    CellValue::Symbol(*text),
                    CellValue::File(FileId(0)),
                    CellValue::Span(Span {
                        start: Offset(0),
                        end: Offset(1),
                    }),
                ]
            })
            .collect(),
        count: RowCount(3),
    };
    let source = Rows::Computed(&table);
    with_context(&mut symbols, |context| {
        let batch = evaluate_rows(
            &prepared,
            Relation::SemanticSyntax,
            &source,
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        let result = batch.class(ClassPlace(0)).unwrap();
        assert_eq!(result.positions.0, [0]);
        assert_eq!(batch.visits(ClassPlace(0)).0, 2);
        assert!(matches!(
            &result.completion,
            Err(OperatingError::InvalidRegex { .. })
        ));
        let columns = [ColumnIndex(2), ColumnIndex(0), ColumnIndex(2)];
        let mut collected = Collected::default();
        match scenario {
            Scenario::PrefixBeforeTheError => {
                let replayed = selected(
                    result,
                    &source,
                    SelectionColumns::Columns(&columns),
                    context,
                    &mut collected,
                );
                assert!(matches!(replayed, Err(OperatingError::InvalidRegex { .. })));
                assert_eq!(
                    collected.0,
                    vec![vec![
                        CellValue::Symbol(valid),
                        CellValue::Node(NodeId(0)),
                        CellValue::Symbol(valid)
                    ]]
                );
                Ok(collected.0.len())
            }
            Scenario::TheTerminalError => selected(
                result,
                &source,
                SelectionColumns::Columns(&columns),
                context,
                &mut collected,
            )
            .map(|()| collected.0.len()),
            Scenario::ASinkThatRefuses => {
                selected(result, &source, SelectionColumns::All, context, &mut Refuse)
                    .map(|()| collected.0.len())
            }
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_accepted_prefix_is_replayed_in_the_projected_columns_before_the_error"),
        word: Word::Succeeds,
        input: || Scenario::PrefixBeforeTheError,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_terminal_predicate_error_follows_the_accepted_prefix"),
        word: Word::Refuses,
        input: || Scenario::TheTerminalError,
        expect: Expect::Refusal(|error| {
            ryo_values::selects!(matches!(error, OperatingError::InvalidRegex { .. }))
        }),
    },
    Case {
        name: CaseName("a_sink_that_refuses_a_row_ends_the_replay_with_its_refusal"),
        word: Word::Fails,
        input: || Scenario::ASinkThatRefuses,
        expect: Expect::Error(OperatingError::NotRead),
    },
];
