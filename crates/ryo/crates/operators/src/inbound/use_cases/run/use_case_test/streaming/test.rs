//! A capture read by one filter streams into it at seam C, through `plan`, `run` and `rows` only: each program run as
//! planned and as its held twin, the two answering the same rows and the same first fault across chunks, the filter's memo
//! growing with the names the capture mints, and every chunk within its cell bound.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::file_buffers::value::ChunkMeter;
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::domain::physical_plan::types::{NodeRows, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::stream_programs::{
    against, faulting, one_row_past_the_bound, select_of, span, split_names,
};
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, agrees, node_probe, ran_after, ran_over_short_rows, ran_program,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, RepeatedRows, StepId, Test,
};
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
    EngineRanTheMiddle,
    ShortRowsProjected,
    ShortRowsFiltered,
    ShortRowProbed,
    OneRowPastTheBound,
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
    /// How many rows the planned run answered, and its fault; none when it answered rows.
    Rows {
        rows: usize,
        fault: Cow<'static, str>,
    },
    /// Whether the two runs answered the same, and the most rows the planned run handed from a stage to its reader at once.
    Handed { same: bool, handed: usize },
    /// Whether the two runs answered the same, how many node tables the planned run formed, and whether the middle node
    /// the engine ran first holds rows after its reader ran.
    Started {
        same: bool,
        formed: u32,
        middle: Cow<'static, str>,
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
    Case {
        name: CaseName("a_node_the_engine_ran_first_starts_the_chain"),
        word: Word::Succeeds,
        input: || Scenario::EngineRanTheMiddle,
        expect: Expect::Value(Observation::Started {
            same: true,
            formed: 2,
            middle: Cow::Borrowed("absent"),
        }),
    },
    Case {
        name: CaseName("a_relations_short_rows_are_each_read_on_their_own"),
        word: Word::Succeeds,
        input: || Scenario::ShortRowsProjected,
        expect: Expect::Value(Observation::Rows {
            rows: 2,
            fault: Cow::Borrowed(""),
        }),
    },
    Case {
        name: CaseName("a_filter_refuses_a_relations_short_row"),
        word: Word::Succeeds,
        input: || Scenario::ShortRowsFiltered,
        expect: Expect::Value(Observation::Rows {
            rows: 0,
            fault: Cow::Borrowed("ShapeMismatch"),
        }),
    },
    Case {
        name: CaseName("a_short_row_kept_into_a_chunk_is_no_row_to_a_probe"),
        word: Word::Succeeds,
        input: || Scenario::ShortRowProbed,
        expect: Expect::Value(Observation::Rows {
            rows: 0,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(0) }"),
        }),
    },
    Case {
        name: CaseName("one_left_rows_answers_are_handed_on_in_one_chunk"),
        word: Word::Succeeds,
        input: || Scenario::OneRowPastTheBound,
        expect: Expect::Value(Observation::Handed {
            same: true,
            handed: 20_001,
        }),
    },
];

/// Runs `steps` over `rows` short Defines rows, emitting the last: the rows it answered and its fault.
fn over_short_rows(steps: &[Step], rows: usize) -> Result<Value, Failure> {
    let emit = PlanNodeId(
        u32::try_from(steps.len())
            .unwrap_or(u32::MAX)
            .saturating_sub(1),
    );
    let ran = ran_over_short_rows(steps, emit, rows)?;
    Ok(Observation::Rows {
        rows: ran.answer.as_ref().map_or(0, Vec::len),
        fault: Cow::Owned(
            ran.answer
                .err()
                .map(|fault| format!("{fault:?}"))
                .unwrap_or_default(),
        ),
    })
}

/// The program each scenario runs; none for those that read short rows or hand a probe's answers on.
fn program(scenario: Scenario) -> Option<Program> {
    Some(match scenario {
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
        // The capture streams into the filter, the filter into the projection; the engine runs the filter first.
        Scenario::EngineRanTheMiddle => Program {
            steps: vec![
                split_names(),
                select_of(0, vec![against(Test::Equal, 7, "a")]),
                over_nodes(
                    Operation::Project {
                        from: StepInput::Step(StepId(1)),
                        columns: vec![ColumnIndex(3), ColumnIndex(8)],
                        distinct: RepeatedRows::Kept,
                    },
                    &[1],
                ),
            ],
            emit: PlanNodeId(2),
            rows: ["abc", "ab", "xab", "abab"]
                .into_iter()
                .map(|name| (name.to_owned(), span(0, 0)))
                .collect(),
            texts: ["abc", "ab", "xab", "abab", "a"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        },
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
        Scenario::ShortRowsProjected
        | Scenario::ShortRowsFiltered
        | Scenario::ShortRowProbed
        | Scenario::OneRowPastTheBound => {
            return None;
        }
    })
}

/// What a scenario with no program observes: the short rows read, or a probe's answers handed on.
fn unprogrammed(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::ShortRowsProjected => over_short_rows(
            &[over_defines(Operation::Project {
                from: StepInput::Relation(Relation::Defines),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            })],
            2,
        ),
        Scenario::ShortRowsFiltered => over_short_rows(
            &[over_defines(Operation::Select {
                from: StepInput::Relation(Relation::Defines),
                comparisons: vec![against(Test::Equal, 0, "a")],
                kept: Vec::new(),
            })],
            2,
        ),
        // A select keeps the short row whole into its chunk; the probe reads it from there as no row.
        Scenario::ShortRowProbed => over_short_rows(
            &[
                over_defines(Operation::Select {
                    from: StepInput::Relation(Relation::Defines),
                    comparisons: Vec::new(),
                    kept: Vec::new(),
                }),
                node_probe(),
            ],
            1,
        ),
        Scenario::OneRowPastTheBound => {
            let program = one_row_past_the_bound();
            let streamed = ran_program(&program, false)?;
            let twin = ran_program(&program, true)?;
            Ok(Observation::Handed {
                same: agrees(&streamed, &twin),
                handed: streamed.buffers.chains.handed.0,
            })
        }
        Scenario::CaptureIntoFilter
        | Scenario::FilterFaultFirst
        | Scenario::MapFaultFirst
        | Scenario::NamesMintedAcrossChunks
        | Scenario::ExpandingCapture
        | Scenario::EngineRanTheMiddle => Err(PortableOperatorFault::WrongOperation),
    }
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let Some(program) = program(scenario) else {
        return unprogrammed(scenario);
    };
    if let Scenario::EngineRanTheMiddle = scenario {
        let streamed = ran_after(&program, false, &[PlanNodeId(1)])?;
        let twin = ran_after(&program, true, &[PlanNodeId(1)])?;
        return Ok(Observation::Started {
            same: agrees(&streamed, &twin),
            formed: streamed.formed,
            middle: Cow::Borrowed(match streamed.buffers.rows(PlanNodeId(1)) {
                NodeRows::Materialized(_) => "held",
                NodeRows::Absent => "absent",
            }),
        });
    }
    let streamed = ran_program(&program, false)?;
    let twin = ran_program(&program, true)?;
    let same = agrees(&streamed, &twin);
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
        Scenario::EngineRanTheMiddle
        | Scenario::ShortRowsProjected
        | Scenario::ShortRowsFiltered
        | Scenario::ShortRowProbed
        | Scenario::OneRowPastTheBound => return Err(PortableOperatorFault::WrongOperation),
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
