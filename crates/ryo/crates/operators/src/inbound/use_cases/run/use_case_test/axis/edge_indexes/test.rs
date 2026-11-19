//! A graph walk and join through the same ordered file key, including deferred acquisition and source faults.
use super::super::test::{EDGES, axis, edges, rows_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::run::fixtures::unshared::emitted_unshared;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted, planned, ran};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, Input as StepInput, JoinColumn, Side, StepScope,
};
use ryo_values::domain::plans::types::Operation;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    AxisFirst,
    ProbeFirst,
    EmptyStart,
    DifferentKey,
    CrossScope,
    ReusedKeyWithInvalidChild,
    RefusedAfterProbe,
}

#[derive(Debug, PartialEq)]
enum Observation {
    Executed {
        builds: usize,
        private_builds: usize,
        ordered: bool,
    },
    Lazy {
        builds: usize,
        opened: usize,
        declared: bool,
    },
    Planned(usize),
    Fault(PortableOperatorFault),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_axis_builds_the_same_index_the_next_join_reads"),
        word: Word::Succeeds,
        input: || Scenario::AxisFirst,
        expect: Expect::Value(Observation::Executed {
            builds: 1,
            private_builds: 2,
            ordered: true,
        }),
    },
    Case {
        name: CaseName("a_join_builds_the_same_index_the_next_axis_reads"),
        word: Word::Succeeds,
        input: || Scenario::ProbeFirst,
        expect: Expect::Value(Observation::Executed {
            builds: 1,
            private_builds: 2,
            ordered: true,
        }),
    },
    Case {
        name: CaseName("an_axis_with_empty_starts_does_not_open_the_shared_index"),
        word: Word::Succeeds,
        input: || Scenario::EmptyStart,
        expect: Expect::Value(Observation::Lazy {
            builds: 0,
            opened: 0,
            declared: true,
        }),
    },
    Case {
        name: CaseName("an_axis_with_another_key_is_not_registered_with_the_probe"),
        word: Word::Succeeds,
        input: || Scenario::DifferentKey,
        expect: Expect::Value(Observation::Planned(0)),
    },
    Case {
        name: CaseName("an_axis_and_a_probe_in_another_scope_do_not_share"),
        word: Word::Succeeds,
        input: || Scenario::CrossScope,
        expect: Expect::Value(Observation::Planned(0)),
    },
    Case {
        name: CaseName("a_ready_index_preserves_the_fault_of_an_invalid_axis_destination"),
        word: Word::Succeeds,
        input: || Scenario::ReusedKeyWithInvalidChild,
        expect: Expect::Value(Observation::Fault(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(99),
        })),
    },
    Case {
        name: CaseName("a_ready_index_does_not_hide_a_refused_fact_source"),
        word: Word::Succeeds,
        input: || Scenario::RefusedAfterProbe,
        expect: Expect::Value(Observation::Fault(PortableOperatorFault::NotRun)),
    },
];

fn probe(column: ColumnIndex) -> Step {
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(Relation::Defines),
            right: StepInput::Relation(EDGES),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![column],
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
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![
            PlanInput::Relation(Relation::Defines),
            PlanInput::Relation(EDGES),
        ],
    }
}

fn two_readers(steps: &[Step; 2], scope: StepScope) -> Result<PhysicalPlan, Failure> {
    let mut program = LogicalProgram::new();
    for (step, scope) in steps.iter().zip([StepScope::PerFile, scope]) {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(0));
    program.emit(RuleOrdinal(1), PlanNodeId(1));
    planned(&program)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut axis_step = axis(Direction::Child);
    if matches!(scenario, Scenario::ReusedKeyWithInvalidChild)
        && let Operation::Axis { child, .. } = &mut axis_step.operation
    {
        *child = ColumnIndex(99);
    }
    let column = if matches!(scenario, Scenario::DifferentKey) {
        ColumnIndex(1)
    } else {
        ColumnIndex(0)
    };
    let steps = [axis_step, probe(column)];
    let scope = if matches!(scenario, Scenario::CrossScope) {
        StepScope::CrossFile
    } else {
        StepScope::PerFile
    };
    let plan = two_readers(&steps, scope)?;
    if matches!(scenario, Scenario::DifferentKey | Scenario::CrossScope) {
        return Ok(Observation::Planned(plan.indexes.len()));
    }
    let mut resources = TextResources::new(&["f"]);
    let graph = edges(&[(1, 3), (1, 2), (2, 4)], &resources);
    let domain = rows_of(
        Relation::Function,
        &[(1, 1), (2, 2), (3, 3), (4, 4)],
        &resources,
    );
    let start_nodes = if matches!(scenario, Scenario::EmptyStart) {
        &[][..]
    } else {
        &[(1, 1)][..]
    };
    let starts = rows_of(Relation::Defines, start_nodes, &resources);
    let inputs = TableInputs::new(&[
        (EDGES, &graph),
        (Relation::Defines, &starts),
        (Relation::Function, &domain),
    ]);
    let mut buffers = FileBuffers::default();
    if matches!(scenario, Scenario::EmptyStart) {
        let answer = emitted(
            &steps,
            &[PlanNodeId(0)],
            &inputs,
            &mut resources,
            &mut buffers,
        )?;
        let declared = answer.first().is_some_and(|row| {
            row.count.0 == 0
                && row.kinds.len() == starts.kinds.len() + 1
                && row.kinds.last() == Some(&CellKind::Node)
        });
        return Ok(Observation::Lazy {
            builds: buffers.builds.0,
            opened: buffers.shared.opened.len(),
            declared,
        });
    }
    if matches!(
        scenario,
        Scenario::ReusedKeyWithInvalidChild | Scenario::RefusedAfterProbe
    ) {
        ran(&plan, PlanNodeId(1), &mut buffers, &inputs, &mut resources)?;
        let absent =
            TableInputs::new(&[(Relation::Defines, &starts), (Relation::Function, &domain)]);
        let axis_input = if matches!(scenario, Scenario::RefusedAfterProbe) {
            &absent
        } else {
            &inputs
        };
        return ran(
            &plan,
            PlanNodeId(0),
            &mut buffers,
            axis_input,
            &mut resources,
        )
        .err()
        .map(Observation::Fault)
        .ok_or(PortableOperatorFault::WrongOperation);
    }
    let emits = if matches!(scenario, Scenario::AxisFirst) {
        [PlanNodeId(0), PlanNodeId(1)]
    } else {
        [PlanNodeId(1), PlanNodeId(0)]
    };
    let shared = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers)?;
    let mut private = FileBuffers::default();
    let own = emitted_unshared(&steps, &emits, &inputs, &mut resources, &mut private)?;
    let ordered = shared == own
        && shared.iter().all(|table| {
            table.count.0 == 2
                && table
                    .cells
                    .chunks_exact(table.kinds.len())
                    .filter_map(|row| row.last().copied())
                    .eq([super::super::test::node(3), super::super::test::node(2)])
        });
    Ok(Observation::Executed {
        builds: buffers.builds.0,
        private_builds: private.builds.0,
        ordered,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
