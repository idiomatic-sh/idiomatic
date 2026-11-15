//! Every streaming stage in one chain: a filter, a projection and extensions behind a capture, each against its held twin;
//! an extension's values resolved before any row and kept per stage, the downstream resolution's fault winning, and a
//! distinct tail dropping repeats across chunks.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, against, agrees, ran_program, select_of, span, split_names,
};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, LiteralText, Operand, ParameterId, QueryLiteral, RepeatedRows,
    StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Chain,
    TwoExtensions,
    DownstreamResolution,
    DownstreamResolutionOverNoRows,
    ResolutionOverFaultingRows,
    AfterResolutionFault,
    DistinctTail,
}

/// Whether the program run as planned answered as its held twin did, how many node tables it formed, and its fault.
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    /// Whether the two runs answered the same.
    same: bool,
    /// How many node tables the planned run formed.
    formed: u32,
    /// The planned run's fault; none when it answered rows.
    fault: Cow<'static, str>,
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_chain_of_single_reader_stages_holds_only_its_tail"),
        word: Word::Succeeds,
        input: || Scenario::Chain,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("two_extensions_in_one_chain_keep_their_own_values"),
        word: Word::Succeeds,
        input: || Scenario::TwoExtensions,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("the_downstream_extension_fault_wins"),
        word: Word::Succeeds,
        input: || Scenario::DownstreamResolution,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("the_downstream_extension_fault_wins_over_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::DownstreamResolutionOverNoRows,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("an_extension_fault_wins_over_its_streamed_inputs"),
        word: Word::Succeeds,
        input: || Scenario::ResolutionOverFaultingRows,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("a_stage_after_a_resolution_fault_raises_no_such_column"),
        word: Word::Succeeds,
        input: || Scenario::AfterResolutionFault,
        expect: Expect::Value(Observation {
            same: true,
            formed: 0,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(2) }"),
        }),
    },
    Case {
        name: CaseName("a_distinct_tail_dedupes_across_chunks"),
        word: Word::Succeeds,
        input: || Scenario::DistinctTail,
        expect: Expect::Value(Observation {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
];

/// An extension of node `from`'s rows by `values`.
fn extend_of(from: u32, values: Vec<Operand>) -> Step {
    over_nodes(
        Operation::Extend {
            from: StepInput::Step(StepId(from)),
            values,
        },
        &[from],
    )
}

/// An extension of Defines' rows by `values`.
fn extend_defines(values: Vec<Operand>) -> Step {
    over_defines(Operation::Extend {
        from: StepInput::Relation(Relation::Defines),
        values,
    })
}

/// A projection of node `from`'s rows to `columns`, repeats dropped or kept.
fn project_of(from: u32, columns: &[u32], distinct: RepeatedRows) -> Step {
    over_nodes(
        Operation::Project {
            from: StepInput::Step(StepId(from)),
            columns: columns.iter().copied().map(ColumnIndex).collect(),
            distinct,
        },
        &[from],
    )
}

/// A written text.
fn text(written: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(written.into())))
}

/// A written number.
fn number(written: i64) -> Operand {
    Operand::Literal(QueryLiteral::Number(Integer(written)))
}

/// A parameter bound to nothing: its resolution faults `Unbound`.
fn unbound() -> Operand {
    Operand::Parameter(ParameterId(0))
}

/// A column: no value an extension resolves, so its resolution faults `WrongBinding`.
fn column() -> Operand {
    Operand::Column(ColumnIndex(0))
}

/// `steps` over Defines rows of `names`, each over no bytes, the last step emitted; `texts` are the file's.
fn over_names(steps: Vec<Step>, names: &[&str], texts: &[&str]) -> Program {
    Program {
        emit: PlanNodeId(
            u32::try_from(steps.len())
                .unwrap_or(u32::MAX)
                .saturating_sub(1),
        ),
        steps,
        rows: names
            .iter()
            .map(|name| ((*name).to_owned(), span(0, 0)))
            .collect(),
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
    }
}

/// The program each scenario runs.
fn program(scenario: Scenario) -> Program {
    let names = ["abc", "ab", "xab", "abab"];
    let texts = ["abc", "ab", "xab", "abab", "a", "b"];
    match scenario {
        Scenario::Chain => over_names(
            vec![
                split_names(),
                select_of(0, vec![against(Test::Equal, 7, "a")]),
                project_of(1, &[3, 8], RepeatedRows::Kept),
                extend_of(2, vec![number(7)]),
            ],
            &names,
            &texts,
        ),
        Scenario::TwoExtensions => over_names(
            vec![
                extend_defines(vec![text("a")]),
                select_of(0, vec![against(Test::Equal, 7, "a")]),
                extend_of(1, vec![text("b"), number(3)]),
            ],
            &names,
            &texts,
        ),
        Scenario::DownstreamResolution => over_names(
            vec![
                extend_defines(vec![column()]),
                extend_of(0, vec![unbound()]),
            ],
            &names,
            &texts,
        ),
        Scenario::DownstreamResolutionOverNoRows => over_names(
            vec![
                over_defines(Operation::Select {
                    from: StepInput::Relation(Relation::Defines),
                    comparisons: vec![against(Test::Equal, 3, "b")],
                    kept: Vec::new(),
                }),
                extend_of(0, vec![column()]),
                extend_of(1, vec![unbound()]),
            ],
            &names,
            &texts,
        ),
        Scenario::ResolutionOverFaultingRows => Program {
            steps: vec![
                over_defines(Operation::SpanBetween {
                    from: StepInput::Relation(Relation::Defines),
                    start: ColumnIndex(6),
                    end: ColumnIndex(6),
                }),
                extend_of(0, vec![unbound()]),
            ],
            emit: PlanNodeId(1),
            rows: (0..5)
                .map(|at| {
                    (
                        "a".to_owned(),
                        if at == 3 { span(2, 1) } else { span(0, 1) },
                    )
                })
                .collect(),
            texts: vec!["a".into()],
        },
        Scenario::AfterResolutionFault => over_names(
            vec![
                extend_defines(vec![unbound()]),
                project_of(0, &[2], RepeatedRows::Kept),
            ],
            &names,
            &texts,
        ),
        Scenario::DistinctTail => {
            // Rows of 8 cells after the span-between stage, over three chunks and a half, their names cycling through
            // three, so a repeat falls either side of every chunk boundary.
            let cycle = ["a", "b", "ab"];
            let rows = STREAM_CHUNK_CELLS / 8 * 7 / 2;
            Program {
                steps: vec![
                    over_defines(Operation::SpanBetween {
                        from: StepInput::Relation(Relation::Defines),
                        start: ColumnIndex(6),
                        end: ColumnIndex(6),
                    }),
                    project_of(0, &[3, 7], RepeatedRows::Dropped),
                ],
                emit: PlanNodeId(1),
                rows: (0..rows)
                    .map(|at| {
                        (
                            cycle
                                .get(at % cycle.len())
                                .copied()
                                .unwrap_or("a")
                                .to_owned(),
                            span(0, 1),
                        )
                    })
                    .collect(),
                texts: cycle.iter().map(|text| (*text).to_owned()).collect(),
            }
        }
    }
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let program = program(scenario);
    let streamed = ran_program(&program, false)?;
    let twin = ran_program(&program, true)?;
    Ok(Observation {
        same: agrees(&streamed, &twin),
        formed: streamed.formed,
        fault: Cow::Owned(
            streamed
                .answer
                .err()
                .map(|fault| format!("{fault:?}"))
                .unwrap_or_default(),
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
