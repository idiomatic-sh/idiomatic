//! A node's rows go once both its node readers and its engine uses are done, whichever comes last, through `plan`, `run`,
//! `rows` and `used` only: an engine use before a later node reader keeps the rows for it, a node reader before the engine
//! use leaves them to it, and a node two rules emit waits for both.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{
    DEFINES, Step, defines_rows, over_defines, over_nodes,
};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, planned, ran};
use ryo_relations::domain::relations::types::{Offset, Span};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, RepeatedRows, StepId, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// The order a case does its runs and uses in.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    UseThenRead,
    ReadThenUse,
    TwoUses,
}
/// Whether node 0 holds rows after each step, in order: `held` or `absent`.
type Value = Cow<'static, str>;
type Input = Scenario;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_engine_use_before_a_later_node_reader_keeps_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::UseThenRead,
        expect: Expect::Value(Cow::Borrowed("held held absent")),
    },
    Case {
        name: CaseName("a_node_reader_before_the_engine_use_leaves_the_rows_to_it"),
        word: Word::Succeeds,
        input: || Scenario::ReadThenUse,
        expect: Expect::Value(Cow::Borrowed("held absent")),
    },
    Case {
        name: CaseName("a_node_two_rules_emit_waits_for_both_uses"),
        word: Word::Succeeds,
        input: || Scenario::TwoUses,
        expect: Expect::Value(Cow::Borrowed("held held absent")),
    },
];

/// One step of a case: a run of a node, or one engine use of node 0.
#[derive(Clone, Copy, Debug)]
enum Done {
    /// The node is run.
    Ran(PlanNodeId),
    /// One engine use of node 0 is done.
    Used,
}

/// Whether node 0 holds rows after each of `done`, over two Defines rows: node 0 every row of them, emitted by `emits` rules,
/// and node 1, when `read` is set, its projection, emitted by a rule of its own.
fn held_after(read: bool, emits: u32, done: &[Done]) -> Result<Value, Failure> {
    let resources = TextResources::new(&["a"]);
    let span = Span {
        start: Offset(0),
        end: Offset(0),
    };
    let table = defines_rows(&[(resources.symbol("a"), span), (resources.symbol("a"), span)]);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    let mut resources = resources;
    let mut steps: Vec<Step> = vec![over_defines(Operation::Select {
        from: StepInput::Relation(DEFINES),
        comparisons: Vec::new(),
        kept: Vec::new(),
    })];
    if read {
        steps.push(over_nodes(
            Operation::Project {
                from: StepInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            &[0],
        ));
    }
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for rule in 0..emits {
        program.emit(RuleOrdinal(rule), PlanNodeId(0));
    }
    if read {
        program.emit(RuleOrdinal(emits), PlanNodeId(1));
    }
    let plan = planned(&program)?;
    let mut buffers = FileBuffers::default();
    let mut held = Vec::new();
    for step in done {
        match step {
            Done::Ran(node) => ran(&plan, *node, &mut buffers, &inputs, &mut resources)?,
            Done::Used => buffers.used(&plan, PlanNodeId(0)),
        }
        held.push(match buffers.rows(PlanNodeId(0)) {
            NodeRows::Materialized(_) => "held",
            NodeRows::Absent => "absent",
        });
    }
    Ok(Cow::Owned(held.join(" ")))
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::UseThenRead => held_after(
            true,
            1,
            &[
                Done::Ran(PlanNodeId(0)),
                Done::Used,
                Done::Ran(PlanNodeId(1)),
            ],
        ),
        Scenario::ReadThenUse => held_after(true, 1, &[Done::Ran(PlanNodeId(1)), Done::Used]),
        Scenario::TwoUses => held_after(
            false,
            2,
            &[Done::Ran(PlanNodeId(0)), Done::Used, Done::Used],
        ),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
