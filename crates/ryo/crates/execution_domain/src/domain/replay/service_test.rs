//! The replay of a selection class result: the accepted prefix first, then the terminal error.
use std::borrow::Cow;

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
    terms::types::{NameText, Symbol},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    rows::types::RowCount,
};

/// Where the replay hands its rows.
#[derive(Clone, Copy, Debug)]
enum Sink {
    /// A sink that keeps every row.
    Collect,
    /// A sink that refuses the first row as `NotRead`.
    Refuse,
}

/// What a row replays: the result of selecting, from three `semantic_syntax` rows whose texts are `x`, `[` and `x`, those whose
/// text matches the regex in the row (the second row's being invalid).
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// What the selection and a replay of it in these columns into a collecting sink observe.
    Observed(&'static [u32]),
    /// A replay of the selection in these columns (all of them when none) into the sink.
    Replay(Option<&'static [u32]>, Sink),
}

/// What the selection kept and a replay handed on.
#[derive(Debug, PartialEq)]
struct Replayed {
    /// The positions the selection accepted, and how many rows it visited.
    positions: Cow<'static, [u32]>,
    visits: u64,
    /// Whether the selection and the replay each ended with the invalid regex.
    invalid_regex: (bool, bool),
    /// The rows the sink was handed.
    rows: Cow<'static, [Cow<'static, [CellValue]>]>,
}

/// What the replay answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Observed(Replayed),
    /// How many rows a replay that completed handed on.
    Handed(usize),
}

type Input = Ask;
type Value = Answer;
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

fn subject(ask: Input) -> Result<Value, Failure> {
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
                    CellValue::Node(NodeId(
                        u64::try_from(at).expect("a fixture row number fits in u64"),
                    )),
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
        let mut collected = Collected::default();
        match ask {
            Ask::Observed(columns) => {
                let columns: Vec<ColumnIndex> = columns.iter().copied().map(ColumnIndex).collect();
                let replayed = selected(
                    result,
                    &source,
                    SelectionColumns::Columns(&columns),
                    context,
                    &mut collected,
                );
                Ok(Answer::Observed(Replayed {
                    positions: Cow::Owned(result.positions.0.clone()),
                    visits: batch.visits(ClassPlace(0)).0,
                    invalid_regex: (
                        matches!(&result.completion, Err(OperatingError::InvalidRegex { .. })),
                        matches!(replayed, Err(OperatingError::InvalidRegex { .. })),
                    ),
                    rows: collected.0.into_iter().map(Cow::Owned).collect(),
                }))
            }
            Ask::Replay(columns, sink) => {
                let columns: Vec<ColumnIndex> = columns
                    .unwrap_or_default()
                    .iter()
                    .copied()
                    .map(ColumnIndex)
                    .collect();
                let selection = if columns.is_empty() {
                    SelectionColumns::All
                } else {
                    SelectionColumns::Columns(&columns)
                };
                match sink {
                    Sink::Collect => selected(result, &source, selection, context, &mut collected),
                    Sink::Refuse => selected(result, &source, selection, context, &mut Refuse),
                }
                .map(|()| Answer::Handed(collected.0.len()))
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

/// The text column, the node column, and the text column again.
const TEXT_NODE_TEXT: &[u32] = &[2, 0, 2];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_accepted_prefix_is_replayed_in_the_projected_columns_before_the_error"),
        word: Word::Succeeds,
        input: || Ask::Observed(TEXT_NODE_TEXT),
        expect: Expect::Value(Answer::Observed(Replayed {
            positions: Cow::Borrowed(&[0]),
            visits: 2,
            invalid_regex: (true, true),
            rows: Cow::Borrowed(&[Cow::Borrowed(&[
                CellValue::Symbol(Symbol(0)),
                CellValue::Node(NodeId(0)),
                CellValue::Symbol(Symbol(0)),
            ])]),
        })),
    },
    Case {
        name: CaseName("the_terminal_predicate_error_follows_the_accepted_prefix"),
        word: Word::Refuses,
        input: || Ask::Replay(Some(TEXT_NODE_TEXT), Sink::Collect),
        expect: Expect::Refusal(|error| {
            ryo_values::selects!(matches!(error, OperatingError::InvalidRegex { .. }))
        }),
    },
    Case {
        name: CaseName("a_sink_that_refuses_a_row_ends_the_replay_with_its_refusal"),
        word: Word::Fails,
        input: || Ask::Replay(None, Sink::Refuse),
        expect: Expect::Error(OperatingError::NotRead),
    },
];
