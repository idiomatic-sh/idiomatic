//! The runner's response to a node the engine answers dormant, through `plan` and `run` only: it holds the empty table of its
//! layout and reads no input; its reads are done, so an input another reader made goes at its last reader that runs; and a
//! dormant set of exactly the nodes with no row answers as the run with every node active.
use super::super::super::super::fixtures::cases::{DEFINES, defines, input_of, named, select};
use super::super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInput, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::plan::interface::PlanUseCase as _;
use crate::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use crate::inbound::use_cases::run::interface::RunUseCase as _;
use crate::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, RepeatedRows, Side, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The projection of a select read by two projections is dormant; it alone is run.
    DormantAlone,
    /// The dormant projection is asked for before the running one.
    DormantFirst,
    /// The running projection is asked for before the dormant one.
    RunningFirst,
    /// Each node with no row in the run with every node active is dormant.
    EmptyNodesDormant,
}
type Input = Scenario;
/// What the run held and read, written out.
type Value = Cow<'static, str>;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dormant_node_holds_its_empty_layout_and_reads_no_input"),
        word: Word::Succeeds,
        input: || Scenario::DormantAlone,
        expect: Expect::Value(Cow::Borrowed(
            "node 0 absent; node 1 absent; node 2 [Symbol] 0; reads 0",
        )),
    },
    Case {
        name: CaseName("a_dormant_reader_releases_an_input_at_its_last_running_reader"),
        word: Word::Succeeds,
        input: || Scenario::DormantFirst,
        expect: Expect::Value(Cow::Borrowed(
            "node 0 absent; node 1 [Symbol] 2; node 2 [Symbol] 0; reads 1",
        )),
    },
    Case {
        name: CaseName("a_dormant_reader_after_the_running_one_releases_their_input"),
        word: Word::Succeeds,
        input: || Scenario::RunningFirst,
        expect: Expect::Value(Cow::Borrowed(
            "node 0 absent; node 1 [Symbol] 2; node 2 [Symbol] 0; reads 1",
        )),
    },
    Case {
        name: CaseName("a_hand_set_dormant_set_answers_as_all_active_where_the_set_is_empty"),
        word: Word::Succeeds,
        input: || Scenario::EmptyNodesDormant,
        expect: Expect::Value(Cow::Borrowed(
            "dormant [0, 2]; same true; reads 1 against 2; Ok([0, 1, 0, 1])",
        )),
    },
];

/// The name column of what node `from` holds.
fn names_of(from: u32) -> Step {
    Step {
        operation: Operation::Project {
            from: input_of(PlanInput::Node(PlanNodeId(from))),
            columns: vec![ColumnIndex(3)],
            distinct: RepeatedRows::Kept,
        },
        inputs: vec![PlanInput::Node(PlanNodeId(from))],
    }
}

/// The rows of node `left` with a row of `DEFINES` of their node, answering the left node.
fn joined(left: u32) -> Step {
    let left = PlanInput::Node(PlanNodeId(left));
    let right = PlanInput::Relation(DEFINES);
    Step {
        operation: Operation::Join {
            left: input_of(left),
            right: input_of(right),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![left, right],
    }
}

/// The rows of node `left`, then those of node `right`.
fn united(left: u32, right: u32) -> Step {
    let (left, right) = (
        PlanInput::Node(PlanNodeId(left)),
        PlanInput::Node(PlanNodeId(right)),
    );
    Step {
        operation: Operation::Union {
            left: input_of(left),
            right: input_of(right),
        },
        inputs: vec![left, right],
    }
}

/// The plan of `steps`, each of `emits` the emitted node of a rule of its own.
fn planned(
    steps: &[Step],
    emits: &[u32],
) -> Result<crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan, Failure> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in (0..).map(RuleOrdinal).zip(emits) {
        program.emit(rule, PlanNodeId(*node));
    }
    PlanUseCaseImpl.plan(&program)
}

/// Whether `node` holds rows in `buffers`, and how many.
fn held(buffers: &FileBuffers, node: u32) -> String {
    match buffers.rows(PlanNodeId(node)) {
        NodeRows::Materialized(table) => format!("{:?} {}", table.kinds, table.count.0),
        NodeRows::Absent => "absent".to_owned(),
    }
}

/// A select of `DEFINES` read by two projections, the second dormant, run in `order`: node 0's rows after, each projection's
/// rows, and the relation's reads.
fn two_readers(order: &[u32]) -> Result<Value, Failure> {
    let texts = TextResources::new(&["kind", "a", "b"]);
    let rows = defines(&[("a", "a"), ("b", "b")], &texts);
    let inputs = TableInputs::new(&[(DEFINES, &rows)])
        .with_dormant(vec![(PlanNodeId(2), vec![CellKind::Symbol])]);
    let steps = [
        select(PlanInput::Relation(DEFINES), Vec::new()),
        names_of(0),
        names_of(0),
    ];
    let plan = planned(&steps, &[1, 2])?;
    let run = RunUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ));
    let mut buffers = FileBuffers::default();
    let mut resources = texts;
    for node in order {
        run.run(
            &plan,
            PlanNodeId(*node),
            &mut buffers,
            &inputs,
            &mut resources,
        )?;
    }
    Ok(Cow::Owned(format!(
        "node 0 {}; node 1 {}; node 2 {}; reads {}",
        held(&buffers, 0),
        held(&buffers, 1),
        held(&buffers, 2),
        inputs.reads(DEFINES)
    )))
}

/// A program whose nodes with no row in the run with every node active are dormant in a second run: whether both answer
/// the same, and what the second answers.
fn empty_nodes_dormant() -> Result<Value, Failure> {
    let texts = TextResources::new(&["kind", "a", "b"]);
    let rows = defines(&[("a", "a"), ("b", "b")], &texts);
    let steps = [
        select(PlanInput::Relation(DEFINES), vec![named("missing")]),
        select(PlanInput::Relation(DEFINES), vec![named("a")]),
        joined(0),
        united(0, 1),
    ];
    let emits = [0, 1, 2, 3].map(PlanNodeId);
    let mut resources = TextResources::new(&["kind", "a", "b"]);
    let active = TableInputs::new(&[(DEFINES, &rows)]);
    let all = emitted(
        &steps,
        &emits,
        &active,
        &mut resources,
        &mut FileBuffers::default(),
    );
    let dormant = all.as_ref().map_or_else(
        |_| Vec::new(),
        |tables| {
            tables
                .iter()
                .zip(emits)
                .filter(|(table, _)| table.count.0 == 0)
                .map(|(table, node)| (node, table.kinds.clone()))
                .collect()
        },
    );
    let set: Vec<u32> = dormant.iter().map(|(node, _)| node.0).collect();
    let gated = TableInputs::new(&[(DEFINES, &rows)]).with_dormant(dormant);
    let some = emitted(
        &steps,
        &emits,
        &gated,
        &mut resources,
        &mut FileBuffers::default(),
    );
    Ok(Cow::Owned(format!(
        "dormant {set:?}; same {}; reads {} against {}; {:?}",
        format!("{all:?}") == format!("{some:?}"),
        gated.reads(DEFINES),
        active.reads(DEFINES),
        some.map(|tables| tables.iter().map(|table| table.count.0).collect::<Vec<_>>())
    )))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::DormantAlone => two_readers(&[2]),
        Scenario::DormantFirst => two_readers(&[2, 1]),
        Scenario::RunningFirst => two_readers(&[1, 2]),
        Scenario::EmptyNodesDormant => empty_nodes_dormant(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
