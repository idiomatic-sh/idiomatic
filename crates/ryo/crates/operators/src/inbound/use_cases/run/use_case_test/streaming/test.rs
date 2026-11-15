//! A capture read by one filter streams into it at seam C, through `plan`, `run` and `rows` only: each program run as
//! planned and as its held twin, the two answering the same rows and the same first fault across chunks, the filter's memo
//! growing with the names the capture mints, and every chunk within its cell bound.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::over_defines;
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::ChunkMeter;
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::inbound::use_cases::run::fixtures::cases::self_matching;
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, against, ran_program, select_of, span, split_names,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, Test};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CaptureIntoFilter,
    FilterFaultFirst,
    MapFaultFirst,
    NamesMintedAcrossChunks,
    ExpandingCapture,
}

/// What a case observed of the program run as planned, against its held twin.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether the two runs answered the same, and how many node tables the planned run formed.
    Streamed { same: bool, formed: u32 },
    /// Whether the two runs answered the same, how many node tables the planned run formed, and the fault it answered.
    Faulted {
        same: bool,
        formed: u32,
        fault: Cow<'static, str>,
    },
    /// Whether the two runs answered the same, how many node tables the planned run formed, and whether every chunk a
    /// stage handed on held less than the bound before the input row that wrote its last rows.
    Bounded {
        same: bool,
        formed: u32,
        within: bool,
    },
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_capture_read_by_one_filter_holds_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::CaptureIntoFilter,
        expect: Expect::Value(Observation::Streamed {
            same: true,
            formed: 1,
        }),
    },
    Case {
        name: CaseName("the_earliest_rows_fault_wins_across_chunks"),
        word: Word::Succeeds,
        input: || Scenario::FilterFaultFirst,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            formed: 1,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
        }),
    },
    Case {
        name: CaseName("the_earliest_rows_fault_wins_across_chunks_the_map_first"),
        word: Word::Succeeds,
        input: || Scenario::MapFaultFirst,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            formed: 1,
            fault: Cow::Borrowed("InvalidSpan"),
        }),
    },
    Case {
        name: CaseName("a_filter_after_a_capture_reads_the_names_it_minted"),
        word: Word::Succeeds,
        input: || Scenario::NamesMintedAcrossChunks,
        expect: Expect::Value(Observation::Streamed {
            same: true,
            formed: 1,
        }),
    },
    Case {
        name: CaseName("a_capture_that_expands_rows_keeps_chunks_within_the_cell_bound"),
        word: Word::Succeeds,
        input: || Scenario::ExpandingCapture,
        expect: Expect::Value(Observation::Bounded {
            same: true,
            formed: 1,
            within: true,
        }),
    },
];

/// Rows of the span-between stage over a filter, `rows` of them, the stage failing on row `map_fault` (its span runs
/// backwards) and the filter on row `filter_fault` (its name is no pattern).
fn faulting(rows: usize, map_fault: usize, filter_fault: usize) -> Program {
    Program {
        steps: vec![
            over_defines(Operation::SpanBetween {
                from: StepInput::Relation(Relation::Defines),
                start: ColumnIndex(6),
                end: ColumnIndex(6),
            }),
            select_of(0, vec![self_matching(3)]),
        ],
        emit: PlanNodeId(1),
        rows: (0..rows)
            .map(|at| {
                (
                    if at == filter_fault { "[" } else { "a" }.to_owned(),
                    if at == map_fault {
                        span(2, 1)
                    } else {
                        span(0, 1)
                    },
                )
            })
            .collect(),
        texts: vec!["a".into(), "[".into()],
    }
}

/// The program each scenario runs.
fn program(scenario: Scenario) -> Program {
    match scenario {
        Scenario::CaptureIntoFilter => Program {
            steps: vec![
                split_names(),
                select_of(0, vec![against(Test::Equal, 7, "a")]),
            ],
            emit: PlanNodeId(1),
            rows: ["abc", "ab", "xab", "abab"]
                .into_iter()
                .map(|name| (name.to_owned(), span(0, 0)))
                .collect(),
            texts: ["abc", "ab", "xab", "abab", "a"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        },
        // The span-between stage writes rows of 8 cells: a fault in its first chunk and one in its third.
        Scenario::FilterFaultFirst => faulting(
            STREAM_CHUNK_CELLS / 8 * 7 / 2,
            STREAM_CHUNK_CELLS / 8 * 5 / 2,
            STREAM_CHUNK_CELLS / 8 * 3 / 4,
        ),
        Scenario::MapFaultFirst => faulting(
            STREAM_CHUNK_CELLS / 8 * 7 / 2,
            STREAM_CHUNK_CELLS / 8 * 3 / 4,
            STREAM_CHUNK_CELLS / 8 * 5 / 2,
        ),
        Scenario::NamesMintedAcrossChunks => {
            // Each name splits a hundred ways, each way minting its two halves, so the file's names grow chunk after chunk
            // while two comparisons of each name, remembered by name, decide which rows the filter keeps.
            let names: Vec<String> = (0..40)
                .map(|at| format!("{at:02}{}", "ab".repeat(50)))
                .collect();
            Program {
                steps: vec![
                    split_names(),
                    select_of(
                        0,
                        vec![
                            against(Test::StartsWith, 7, "0"),
                            against(Test::EndsWith, 8, "b"),
                        ],
                    ),
                ],
                emit: PlanNodeId(1),
                rows: names
                    .iter()
                    .map(|name| (name.clone(), span(0, 0)))
                    .collect(),
                texts: names,
            }
        }
        Scenario::ExpandingCapture => {
            let name = "x".repeat(2_001);
            Program {
                steps: vec![
                    split_names(),
                    select_of(0, vec![against(Test::Equal, 7, "x")]),
                ],
                emit: PlanNodeId(1),
                rows: (0..200).map(|_| (name.clone(), span(0, 0))).collect(),
                texts: vec![name, "x".into()],
            }
        }
    }
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let program = program(scenario);
    let streamed = ran_program(&program, false)?;
    let twin = ran_program(&program, true)?;
    let same = streamed.answer == twin.answer;
    Ok(match scenario {
        Scenario::CaptureIntoFilter | Scenario::NamesMintedAcrossChunks => Observation::Streamed {
            same,
            formed: streamed.formed,
        },
        Scenario::FilterFaultFirst | Scenario::MapFaultFirst => Observation::Faulted {
            same,
            formed: streamed.formed,
            fault: Cow::Owned(
                streamed
                    .answer
                    .err()
                    .map(|fault| format!("{fault:?}"))
                    .unwrap_or_default(),
            ),
        },
        Scenario::ExpandingCapture => Observation::Bounded {
            same,
            formed: streamed.formed,
            within: streamed.buffers.chunks == ChunkMeter::Within
                && streamed.buffers.stream.levels.iter().all(|chain| {
                    chain
                        .stages
                        .iter()
                        .all(|stage| stage.out.cells.capacity() <= STREAM_CHUNK_CELLS)
                }),
        },
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
