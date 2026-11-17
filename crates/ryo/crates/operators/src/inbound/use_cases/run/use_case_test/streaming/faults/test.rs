//! A chain's faults at seam C, each against its held twin: the earliest row's fault wins whatever the chunk it falls in
//! (the scenarios of the scope's Gherkin, 2,500 rows over the 2,048-row chunks of the span-between stage); a probe's hard
//! fault on its first left row comes before a later stage's preparation fault, the held plan's first, though the later
//! stage is prepared first; and a faulting tail is fed every row its input makes, as its held twin and the base are. A
//! filter's fault in the middle of a chunk keeps the rows before it, and a probe's hard fault on a left row in the middle of
//! a chunk, after a later stage stopped, still wins in node order.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::cases::compared;
use crate::inbound::use_cases::run::fixtures::stream_programs::{
    faulting, probe_before_a_stopped_filter, span,
};
use crate::inbound::use_cases::run::fixtures::streams::{Outcome, Program, agrees, ran_program};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, LiteralText, Operand, QueryLiteral, RepeatedRows,
    Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    FilterBeforeTheMapAcrossChunks,
    MapBeforeTheFilterAcrossChunks,
    FilterPastTheBoundaryBeforeTheMap,
    ProbeBeforeALaterPreparation,
    FaultingTailFeed,
    LaterRightAfterAPreparationFault,
    FilterMidChunk,
    ProbeHardFaultMidChunk,
    StoppedStageFeed,
    ProbeReadsOnAfterAStoppedFilter,
}

/// What a case observed.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether the two runs answered the same, and the rows the emit held before its fault, as planned and in the twin.
    Held {
        same: bool,
        planned: usize,
        twin: usize,
    },
    /// Whether the two runs answered the same, and the planned run's fault.
    Faulted {
        same: bool,
        fault: Cow<'static, str>,
    },
    /// The rows the tail's input fed it as planned, and in the held twin.
    Fed { planned: usize, twin: usize },
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_fault_on_an_earlier_row_wins_over_the_maps_in_the_next_chunk"),
        word: Word::Succeeds,
        input: || Scenario::FilterBeforeTheMapAcrossChunks,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
        }),
    },
    Case {
        name: CaseName("a_map_fault_on_an_earlier_row_wins_over_the_filters_in_the_next_chunk"),
        word: Word::Succeeds,
        input: || Scenario::MapBeforeTheFilterAcrossChunks,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed("InvalidSpan"),
        }),
    },
    Case {
        name: CaseName("a_filter_fault_past_a_chunk_boundary_wins_over_the_maps_after_it"),
        word: Word::Succeeds,
        input: || Scenario::FilterPastTheBoundaryBeforeTheMap,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
        }),
    },
    Case {
        name: CaseName("a_probes_hard_fault_comes_before_a_later_stages_preparation_fault"),
        word: Word::Succeeds,
        input: || Scenario::ProbeBeforeALaterPreparation,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(99) }"),
        }),
    },
    Case {
        name: CaseName("a_faulting_tail_is_fed_every_row_its_input_makes"),
        word: Word::Succeeds,
        input: || Scenario::FaultingTailFeed,
        expect: Expect::Value(Observation::Fed {
            planned: 10,
            twin: 10,
        }),
    },
    Case {
        name: CaseName("a_later_probes_right_is_not_asked_after_a_stage_before_it_faults"),
        word: Word::Succeeds,
        input: || Scenario::LaterRightAfterAPreparationFault,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(0) }"),
        }),
    },
    Case {
        name: CaseName("a_fault_mid_chunk_keeps_the_rows_before_it"),
        word: Word::Succeeds,
        input: || Scenario::FilterMidChunk,
        expect: Expect::Value(Observation::Held {
            same: true,
            planned: 700,
            twin: 700,
        }),
    },
    Case {
        name: CaseName("a_running_probe_reads_on_after_a_later_filter_faults_mid_chunk"),
        word: Word::Succeeds,
        input: || Scenario::ProbeReadsOnAfterAStoppedFilter,
        expect: Expect::Value(Observation::Fed {
            planned: 14_040,
            twin: 14_040,
        }),
    },
    Case {
        name: CaseName("a_probe_hard_fault_after_a_later_stage_stopped_wins"),
        word: Word::Succeeds,
        input: || Scenario::ProbeHardFaultMidChunk,
        expect: Expect::Value(Observation::Faulted {
            same: true,
            fault: Cow::Borrowed("NoSuchColumn { index: ColumnIndex(99) }"),
        }),
    },
    Case {
        name: CaseName("a_stopped_stage_before_the_tail_ends_the_heads_rows"),
        word: Word::Succeeds,
        input: || Scenario::StoppedStageFeed,
        expect: Expect::Value(Observation::Fed {
            planned: 2_048,
            twin: 2_500,
        }),
    },
];

/// 2,500 rows of the span-between stage (node 0) over the filter (node 1), which faults on row 100, and an extension of the
/// filter (node 2): the filter stops in the stage's first chunk of 2,048 rows, before the tail, so the head reads no row
/// past that chunk and the filter is fed that chunk's rows; the held twin's filter is fed every row.
fn stopped_before_the_tail() -> Program {
    let mut program = faulting(2_500, usize::MAX, 100);
    program.steps.push(over_nodes(
        Operation::Extend {
            from: StepInput::Step(StepId(1)),
            values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
        },
        &[1],
    ));
    program.emit = PlanNodeId(2);
    program
}

/// 1,024 Defines rows, those from row 900 named `late`. A select of the rows named `late` (node 0); an inner join of it with
/// Defines keyed on a right column past Defines' rows (node 1), whose index build hard-faults at its first left row, the
/// head's row 900, in the middle of the head's chunk; a projection of a column past the join's (node 2), whose preparation
/// stops it before any row though it is not the tail; and an extension of the projection (node 3).
fn probe_after_a_stopped_stage() -> Program {
    let mut program = probe_then_projection();
    program.steps.push(over_nodes(
        Operation::Extend {
            from: StepInput::Step(StepId(2)),
            values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
        },
        &[2],
    ));
    program.emit = PlanNodeId(3);
    if let Some(Step {
        operation: Operation::Select { comparisons, .. },
        ..
    }) = program.steps.first_mut()
    {
        comparisons.push(compared(
            Test::Equal,
            Operand::Column(ColumnIndex(3)),
            Operand::Literal(QueryLiteral::Text(LiteralText("late".into()))),
        ));
    }
    program.rows = (0..1_024)
        .map(|at| (if at < 900 { "a" } else { "late" }.to_owned(), span(0, 0)))
        .collect();
    program.texts = vec!["a".into(), "late".into()];
    program
}

/// A select of every Defines row (node 0); an inner join of it with Defines keyed on a right column past Defines' rows
/// (node 1), whose index build faults at its first left row; and a projection of a column past the join's (node 2), whose
/// preparation faults.
fn probe_then_projection() -> Program {
    let defines = (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    Program {
        steps: vec![
            over_defines(Operation::Select {
                from: defines.0,
                comparisons: Vec::new(),
                kept: Vec::new(),
            }),
            Step {
                operation: Operation::Join {
                    left: StepInput::Step(StepId(0)),
                    right: defines.0,
                    left_on: vec![ColumnIndex(3)],
                    right_on: vec![ColumnIndex(99)],
                    output: vec![JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    }],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                },
                inputs: vec![PlanInput::Node(PlanNodeId(0)), defines.1],
            },
            over_nodes(
                Operation::Project {
                    from: StepInput::Step(StepId(1)),
                    columns: vec![ColumnIndex(50)],
                    distinct: RepeatedRows::Kept,
                },
                &[1],
            ),
        ],
        emit: PlanNodeId(2),
        rows: ["a", "b"]
            .into_iter()
            .map(|name| (name.to_owned(), span(0, 0)))
            .collect(),
        texts: vec!["a".into(), "b".into()],
    }
}

/// The review's input (Codex, round 1): one Defines row named `[`. A select of Defines whose name matches itself (node 0)
/// faults on it; an inner join of Defines with that select (node 1) meets its right side's fault before any row, so it
/// writes no column; a projection of its column 0 (node 2) faults in its preparation; an inner join of the projection with
/// Comment (node 3), which the inputs refuse, is never reached in the held plan, so Comment is never asked.
fn later_right_after_a_preparation_fault() -> Program {
    let defines = (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    let comment = (
        StepInput::Relation(Relation::Comment),
        PlanInput::Relation(Relation::Comment),
    );
    let join = |left: (StepInput, PlanInput), right: (StepInput, PlanInput), on: u32| Step {
        operation: Operation::Join {
            left: left.0,
            right: right.0,
            left_on: vec![ColumnIndex(on)],
            right_on: vec![ColumnIndex(on)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![left.1, right.1],
    };
    let node = |at: u32| (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)));
    Program {
        steps: vec![
            over_defines(Operation::Select {
                from: defines.0,
                comparisons: vec![
                    crate::inbound::use_cases::run::fixtures::cases::self_matching(3),
                ],
                kept: Vec::new(),
            }),
            join(defines, node(0), 3),
            over_nodes(
                Operation::Project {
                    from: StepInput::Step(StepId(1)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
                &[1],
            ),
            join(node(2), comment, 0),
        ],
        emit: PlanNodeId(3),
        rows: vec![("[".to_owned(), span(0, 0))],
        texts: vec!["[".into()],
    }
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let program = match scenario {
        Scenario::FilterBeforeTheMapAcrossChunks => faulting(2_500, 2_100, 1_500),
        Scenario::MapBeforeTheFilterAcrossChunks => faulting(2_500, 1_500, 2_100),
        Scenario::FilterPastTheBoundaryBeforeTheMap => faulting(2_500, 2_100, 2_060),
        Scenario::ProbeBeforeALaterPreparation => probe_then_projection(),
        Scenario::FaultingTailFeed => faulting(10, usize::MAX, 5),
        Scenario::LaterRightAfterAPreparationFault => later_right_after_a_preparation_fault(),
        Scenario::FilterMidChunk => faulting(1_024, usize::MAX, 700),
        Scenario::ProbeHardFaultMidChunk => probe_after_a_stopped_stage(),
        Scenario::StoppedStageFeed => stopped_before_the_tail(),
        Scenario::ProbeReadsOnAfterAStoppedFilter => probe_before_a_stopped_filter(),
    };
    let planned = ran_program(&program, false)?;
    let twin = ran_program(&program, true)?;
    if let Scenario::FilterMidChunk = scenario {
        return Ok(Observation::Held {
            same: agrees(&planned, &twin),
            planned: planned.held,
            twin: twin.held,
        });
    }
    // The node whose feed a feed case reads: the stopped filter after the running probe, or the tail.
    let fed_node = match scenario {
        Scenario::ProbeReadsOnAfterAStoppedFilter => Some(PlanNodeId(2)),
        Scenario::FaultingTailFeed | Scenario::StoppedStageFeed => Some(PlanNodeId(1)),
        Scenario::FilterBeforeTheMapAcrossChunks
        | Scenario::MapBeforeTheFilterAcrossChunks
        | Scenario::FilterPastTheBoundaryBeforeTheMap
        | Scenario::ProbeBeforeALaterPreparation
        | Scenario::LaterRightAfterAPreparationFault
        | Scenario::FilterMidChunk
        | Scenario::ProbeHardFaultMidChunk => None,
    };
    if let Some(node) = fed_node {
        let fed =
            |buffers: &crate::domain::physical_plan::types::file_buffers::value::FileBuffers| {
                buffers.fed(node).0.first().map_or(0, |rows| rows.0)
            };
        return Ok(Observation::Fed {
            planned: fed(&planned.buffers),
            twin: fed(&twin.buffers),
        });
    }
    Ok(Observation::Faulted {
        same: agrees(&planned, &twin),
        // The engine's outcome: a refused relation before the operators' fault.
        fault: Cow::Owned(
            planned
                .outcome
                .err()
                .map(|outcome| match outcome {
                    Outcome::Fault(fault) => format!("{fault:?}"),
                    Outcome::Refused(relation) => format!("Refused({relation:?})"),
                })
                .unwrap_or_default(),
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
