//! The planner streams a node only when its one reader is a streaming stage that reads it first and no engine use keeps it;
//! each condition broken alone holds it.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::service::plan;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeFlow, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, JoinColumn, RepeatedRows, Side, StepId,
    StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    CaptureIntoFilter,
    NoStreamingStage,
    TwoReaders,
    Emitted,
    Held,
    ReaderBlocks,
    ReadSecond,
    DroppingRepeatsInTheMiddle,
    ProbeDroppingRepeatsInTheMiddle,
}
/// Each node's flow, in node order: `S` streamed, `H` held.
type Value = Cow<'static, str>;
type Input = Scenario;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_capture_read_by_one_filter_streams"),
        word: Word::Succeeds,
        input: || Scenario::CaptureIntoFilter,
        expect: Expect::Value(Cow::Borrowed("SH")),
    },
    Case {
        name: CaseName("a_node_that_is_no_streaming_stage_is_held"),
        word: Word::Succeeds,
        input: || Scenario::NoStreamingStage,
        expect: Expect::Value(Cow::Borrowed("HH")),
    },
    Case {
        name: CaseName("a_node_with_two_readers_is_held"),
        word: Word::Succeeds,
        input: || Scenario::TwoReaders,
        expect: Expect::Value(Cow::Borrowed("HHH")),
    },
    Case {
        name: CaseName("an_emitted_middle_node_is_held"),
        word: Word::Succeeds,
        input: || Scenario::Emitted,
        expect: Expect::Value(Cow::Borrowed("HH")),
    },
    Case {
        name: CaseName("a_held_middle_node_is_held"),
        word: Word::Succeeds,
        input: || Scenario::Held,
        expect: Expect::Value(Cow::Borrowed("HH")),
    },
    Case {
        name: CaseName("a_node_read_by_a_blocking_kernel_is_held"),
        word: Word::Succeeds,
        input: || Scenario::ReaderBlocks,
        expect: Expect::Value(Cow::Borrowed("HH")),
    },
    Case {
        name: CaseName("a_node_read_as_a_probes_right_is_held"),
        word: Word::Succeeds,
        input: || Scenario::ReadSecond,
        expect: Expect::Value(Cow::Borrowed("SHH")),
    },
    Case {
        name: CaseName("a_projection_dropping_repeats_streams_only_as_a_tail"),
        word: Word::Succeeds,
        input: || Scenario::DroppingRepeatsInTheMiddle,
        expect: Expect::Value(Cow::Borrowed("SHH")),
    },
    Case {
        name: CaseName("a_probe_dropping_repeats_streams_only_as_a_tail"),
        word: Word::Succeeds,
        input: || Scenario::ProbeDroppingRepeatsInTheMiddle,
        expect: Expect::Value(Cow::Borrowed("SHH")),
    },
];

/// A capture of each name of Defines.
fn capture() -> Step {
    over_defines(Operation::Capture {
        from: StepInput::Relation(Relation::Defines),
        path: ColumnIndex(3),
        pattern: CapturePattern("{x}".into()),
    })
}

/// A step of `operation` reading node 0, then Defines.
fn over_nodes_and_defines(operation: Operation) -> Step {
    Step {
        operation,
        inputs: vec![
            PlanInput::Node(PlanNodeId(0)),
            PlanInput::Relation(Relation::Defines),
        ],
    }
}

/// A select of every row of node `from`.
fn select_of(from: u32) -> Step {
    over_nodes(
        Operation::Select {
            from: StepInput::Step(StepId(from)),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        &[from],
    )
}

/// The flows of `steps` planned with the last step emitted, and `emitted` and `held` emitted and held as well.
fn flows(steps: &[Step], emitted: &[u32], held: &[u32]) -> Result<Value, Failure> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    let last = u32::try_from(steps.len())
        .unwrap_or(u32::MAX)
        .saturating_sub(1);
    for (rule, node) in std::iter::once(last)
        .chain(emitted.iter().copied())
        .enumerate()
    {
        program.emit(
            RuleOrdinal(u32::try_from(rule).unwrap_or(u32::MAX)),
            PlanNodeId(node),
        );
    }
    for node in held {
        program.hold(PlanNodeId(*node));
    }
    let plan = plan(&program)?;
    Ok(Cow::Owned(
        plan.nodes
            .iter()
            .map(|node| match node.flow {
                NodeFlow::Streamed => 'S',
                NodeFlow::Held => 'H',
            })
            .collect(),
    ))
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::CaptureIntoFilter => flows(&[capture(), select_of(0)], &[], &[]),
        Scenario::NoStreamingStage => flows(
            &[
                over_defines(Operation::GroupCount {
                    from: StepInput::Relation(Relation::Defines),
                    keys: vec![ColumnIndex(3)],
                    distinct: None,
                }),
                select_of(0),
            ],
            &[],
            &[],
        ),
        Scenario::TwoReaders => flows(&[capture(), select_of(0), select_of(0)], &[1], &[]),
        Scenario::Emitted => flows(&[capture(), select_of(0)], &[0], &[]),
        Scenario::Held => flows(&[capture(), select_of(0)], &[], &[0]),
        Scenario::ReaderBlocks => flows(
            &[
                capture(),
                over_nodes(
                    Operation::GroupCount {
                        from: StepInput::Step(StepId(0)),
                        keys: vec![ColumnIndex(3)],
                        distinct: None,
                    },
                    &[0],
                ),
            ],
            &[],
            &[],
        ),
        Scenario::DroppingRepeatsInTheMiddle => flows(
            &[
                capture(),
                over_nodes(
                    Operation::Project {
                        from: StepInput::Step(StepId(0)),
                        columns: vec![ColumnIndex(3)],
                        distinct: RepeatedRows::Dropped,
                    },
                    &[0],
                ),
                select_of(1),
            ],
            &[],
            &[],
        ),
        Scenario::ProbeDroppingRepeatsInTheMiddle => flows(
            &[
                capture(),
                over_nodes_and_defines(Operation::Intersection {
                    left: StepInput::Step(StepId(0)),
                    right: StepInput::Relation(Relation::Defines),
                }),
                select_of(1),
            ],
            &[],
            &[],
        ),
        Scenario::ReadSecond => flows(
            &[
                over_defines(Operation::Select {
                    from: StepInput::Relation(Relation::Defines),
                    comparisons: Vec::new(),
                    kept: Vec::new(),
                }),
                capture(),
                over_nodes(
                    Operation::Join {
                        left: StepInput::Step(StepId(0)),
                        right: StepInput::Step(StepId(1)),
                        left_on: vec![ColumnIndex(3)],
                        right_on: vec![ColumnIndex(7)],
                        output: vec![JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        }],
                        filter: Vec::new(),
                        view: Vec::new(),
                        fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
                        reach: None,
                    },
                    &[0, 1],
                ),
            ],
            &[],
            &[],
        ),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
