//! A chain's faults at seam C, each against its held twin: the earliest row's fault wins whatever the chunk it falls in
//! (the scenarios of the scope's Gherkin, 2,500 rows over the 2,048-row chunks of the span-between stage); a probe's hard
//! fault on its first left row comes before a later stage's preparation fault, the held plan's first, though the later
//! stage is prepared first; and a faulting tail is fed every row its input makes, as its held twin and the base are.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, agrees, faulting, ran_program, span,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, RepeatedRows, Side, StepId,
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
}

/// What a case observed.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
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
];

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

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let program = match scenario {
        Scenario::FilterBeforeTheMapAcrossChunks => faulting(2_500, 2_100, 1_500),
        Scenario::MapBeforeTheFilterAcrossChunks => faulting(2_500, 1_500, 2_100),
        Scenario::FilterPastTheBoundaryBeforeTheMap => faulting(2_500, 2_100, 2_060),
        Scenario::ProbeBeforeALaterPreparation => probe_then_projection(),
        Scenario::FaultingTailFeed => faulting(10, usize::MAX, 5),
    };
    let planned = ran_program(&program, false)?;
    let twin = ran_program(&program, true)?;
    if let Scenario::FaultingTailFeed = scenario {
        let fed =
            |buffers: &crate::domain::physical_plan::types::file_buffers::value::FileBuffers| {
                buffers
                    .fed(PlanNodeId(1))
                    .0
                    .first()
                    .map_or(0, |rows| rows.0)
            };
        return Ok(Observation::Fed {
            planned: fed(&planned.buffers),
            twin: fed(&twin.buffers),
        });
    }
    Ok(Observation::Faulted {
        same: agrees(&planned, &twin),
        fault: Cow::Owned(
            planned
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
