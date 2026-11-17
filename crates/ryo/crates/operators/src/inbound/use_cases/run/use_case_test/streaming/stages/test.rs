//! Every streaming stage in one chain: a filter, a projection and extensions behind a capture, each against its held twin;
//! an extension's values resolved before any row and kept per stage, the downstream resolution's fault winning, a
//! distinct tail dropping repeats across chunks, each stage called once for each chunk of rows and never once for each row,
//! and a capture reading the tree's rows minting names between its chunks.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::stream_programs::{
    against, select_of, span, split_names, three_stages,
};
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, Ran, agrees, ran_over_tree, ran_program,
};
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, LiteralText, Operand, ParameterId,
    QueryLiteral, RepeatedRows, StepId, Test,
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
    ChunkedChain,
    CaptureOfTheTree,
}

/// What a case observed of the program run as planned, against its held twin.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether the two runs answered the same, how many node tables the planned run formed, and its fault; none when it
    /// answered rows.
    Answered {
        same: bool,
        formed: u32,
        fault: Cow<'static, str>,
    },
    /// Whether the two runs answered the same, and whether the planned run's first file called its chain's stages no more
    /// than 14 times: each stage at most once for each chunk handed to it and once more after each chunk it filled.
    Chunked { same: bool, once_per_chunk: bool },
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_chain_of_single_reader_stages_holds_only_its_tail"),
        word: Word::Succeeds,
        input: || Scenario::Chain,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("two_extensions_in_one_chain_keep_their_own_values"),
        word: Word::Succeeds,
        input: || Scenario::TwoExtensions,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("the_downstream_extension_fault_wins"),
        word: Word::Succeeds,
        input: || Scenario::DownstreamResolution,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("the_downstream_extension_fault_wins_over_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::DownstreamResolutionOverNoRows,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("an_extension_fault_wins_over_its_streamed_inputs"),
        word: Word::Succeeds,
        input: || Scenario::ResolutionOverFaultingRows,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("Unbound"),
        }),
    },
    Case {
        name: CaseName("a_stage_after_a_resolution_fault_raises_no_such_column"),
        word: Word::Succeeds,
        input: || Scenario::AfterResolutionFault,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 0,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(2) }"),
        }),
    },
    Case {
        name: CaseName("a_distinct_tail_dedupes_across_chunks"),
        word: Word::Succeeds,
        input: || Scenario::DistinctTail,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("a_chain_calls_each_stage_once_per_chunk"),
        word: Word::Succeeds,
        input: || Scenario::ChunkedChain,
        expect: Expect::Value(Observation::Chunked {
            same: true,
            once_per_chunk: true,
        }),
    },
    Case {
        name: CaseName("a_text_map_head_over_the_tree_mints_between_chunks"),
        word: Word::Succeeds,
        input: || Scenario::CaptureOfTheTree,
        expect: Expect::Value(Observation::Answered {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(""),
        }),
    },
];

/// The most stage calls a chain of three stages over 10,000 rows of 2 cells makes in one file: 2 head chunks of 16 Ki cells,
/// so each stage is handed at most 2 chunks and fills at most 2 (no stage writes more than 30,000 cells), 4 calls each, and
/// at most 2 more for the chunks left at the end of the head. A chain that calls a stage once for each row makes 30,000.
const CHUNKED_CALLS: usize = 14;

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

/// A capture of each tree row's text into `{a}{b}` (node 0) and a select of the rows whose `a` starts with `0` (node 1), over
/// the rows of the wide tree's every node, run as planned or as its held twin.
fn captured_tree(twin: bool) -> Result<Ran, PortableOperatorFault> {
    let width = u32::try_from(kinds(Relation::SyntaxTree).len()).unwrap_or(u32::MAX);
    let capture = Step {
        operation: Operation::Capture {
            from: StepInput::Relation(Relation::SyntaxTree),
            path: ColumnIndex(2),
            pattern: CapturePattern("{a}{b}".into()),
        },
        inputs: vec![PlanInput::Relation(Relation::SyntaxTree)],
    };
    let select = select_of(0, vec![against(Test::StartsWith, width, "0")]);
    ran_over_tree(&[capture, select], PlanNodeId(1), twin, &|_, _| Vec::new())
}

/// The program over Defines rows each scenario runs; none for the capture of the tree, which reads the tree's rows.
fn program(scenario: Scenario) -> Option<Program> {
    let names = ["abc", "ab", "xab", "abab"];
    let texts = ["abc", "ab", "xab", "abab", "a", "b"];
    Some(match scenario {
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
        Scenario::ChunkedChain => three_stages(10_000),
        Scenario::CaptureOfTheTree => return None,
    })
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let (streamed, twin) = match program(scenario) {
        Some(program) => (ran_program(&program, false)?, ran_program(&program, true)?),
        None => (captured_tree(false)?, captured_tree(true)?),
    };
    if let Scenario::ChunkedChain = scenario {
        return Ok(Observation::Chunked {
            same: agrees(&streamed, &twin),
            once_per_chunk: streamed.stage_calls <= CHUNKED_CALLS,
        });
    }
    Ok(Observation::Answered {
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
