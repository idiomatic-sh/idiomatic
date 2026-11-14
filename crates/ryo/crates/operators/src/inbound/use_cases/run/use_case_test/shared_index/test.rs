//! The shared probe index's cases, through the plan and run use cases: probes of one right relation and key build one index
//! per file, and answer what each probe's own index answers, file after file.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{DEFINES, Step, defines_rows};
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, IndexBuilds};
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    LogicalNode, Lowering, NodeBindings, NodeRows, PlanInput, PlanInputList, PlanNodeId,
    RowSource, RuleOrdinal,
};
use crate::inbound::use_cases::run::fixtures::texts::span;
use crate::inbound::use_cases::run::fixtures::unshared::unshared;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, planned, ran};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The three probes over one file, planned as the planner plans them.
    Builds,
    /// The planted counter-example: the same probes, each with its own index.
    BuildsUnshared,
    /// The three probes over two files on one worker's buffers, the first probe released before the others run, shared
    /// against each with its own index.
    SharedAgainstOwn,
    /// The planted counter-example: the last probe reading other right rows, still reading the shared index.
    SharedAcrossRows,
}

/// What a case observed.
#[derive(Debug, PartialEq)]
enum Observation {
    /// How many probe indexes the run built.
    Builds(IndexBuilds),
    /// Whether the two plans answered the same rows.
    Same(bool),
}

type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("probes_of_one_right_side_read_it_once"),
        word: Word::Succeeds,
        input: || Scenario::Builds,
        expect: Expect::Value(Observation::Builds(IndexBuilds(1))),
    },
    Case {
        name: CaseName("probes_with_their_own_indexes_fail_probes_of_one_right_side_read_it_once"),
        word: Word::Succeeds,
        input: || Scenario::BuildsUnshared,
        expect: Expect::Value(Observation::Builds(IndexBuilds(3))),
    },
    Case {
        name: CaseName("a_shared_index_answers_what_its_own_index_answers"),
        word: Word::Succeeds,
        input: || Scenario::SharedAgainstOwn,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("an_index_shared_across_right_rows_fails_a_shared_index_answers_what_its_own_index_answers"),
        word: Word::Succeeds,
        input: || Scenario::SharedAcrossRows,
        expect: Expect::Value(Observation::Same(false)),
    },
];

/// The texts of every file's cells.
const TEXTS: &[&str] = &["n1", "n2", "a"];

/// The probes, the nodes they emit: three joins of other left rows and other answers, each probing `DEFINES` on its name.
const PROBES: [PlanNodeId; 3] = [PlanNodeId(2), PlanNodeId(3), PlanNodeId(4)];

/// A join of `left`, keyed on `left_on`, with `DEFINES` on its name, answering `output`.
fn probe_of(left: (StepInput, PlanInput), left_on: u32, output: &[(Side, u32)]) -> Step {
    Step {
        operation: Operation::Join {
            left: left.0,
            right: StepInput::Relation(DEFINES),
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(3)],
            output: output
                .iter()
                .map(|(side, column)| JoinColumn {
                    side: *side,
                    column: ColumnIndex(*column),
                })
                .collect(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![left.1, PlanInput::Relation(DEFINES)],
    }
}

/// The program: a select and a projection of `DEFINES`, then the three probes, of `DEFINES`, of the select and of the
/// projection.
fn steps() -> Vec<Step> {
    let defines = (StepInput::Relation(DEFINES), PlanInput::Relation(DEFINES));
    let node = |at: u32| (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)));
    vec![
        Step {
            operation: Operation::Select {
                from: defines.0,
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("n1".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
            inputs: vec![defines.1],
        },
        Step {
            operation: Operation::Project {
                from: defines.0,
                columns: vec![ColumnIndex(3), ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            inputs: vec![defines.1],
        },
        probe_of(defines, 3, &[(Side::Left, 0), (Side::Right, 4)]),
        probe_of(node(0), 3, &[(Side::Right, 0)]),
        probe_of(node(1), 0, &[(Side::Left, 1), (Side::Right, 6), (Side::Right, 0)]),
    ]
}

/// The plan of `steps`, each probe a rule's emit.
fn plan_of(steps: &[Step]) -> Result<PhysicalPlan, Failure> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in (0..).map(RuleOrdinal).zip(PROBES) {
        program.emit(rule, node);
    }
    planned(&program)
}

/// `plan` with the last probe reading the select's rows on its right, its index unchanged.
fn reread(mut plan: PhysicalPlan) -> PhysicalPlan {
    if let Some(Lowering::Probe { right, .. }) = plan.nodes.get_mut(4).map(|node| &mut node.lowering) {
        *right = RowSource::Node(PlanNodeId(0));
    }
    plan
}

/// The rows of each file, by its names: each row its own node, its kind, refinement and name the name.
const FILES: [&[&str]; 2] = [&["n1", "n2", "n1", "a"], &["n2", "a", "n2"]];

/// Each file's answers of the three probes, in order, over one worker's buffers: the first probe released before the others
/// run, every node released at the file's end; and how many probe indexes were built.
///
/// # Errors
///
/// The first fault of a probe.
fn answers(plan: &PhysicalPlan, files: &[&[&str]]) -> Result<(Vec<Table>, IndexBuilds), Failure> {
    let mut buffers = FileBuffers::default();
    let mut resources = TextResources::new(TEXTS);
    let mut tables = Vec::with_capacity(files.len() * PROBES.len());
    for names in files {
        let rows: Vec<_> = names
            .iter()
            .map(|name| (resources.symbol(name), span(0, 1)))
            .collect();
        let table = defines_rows(&rows);
        let inputs = TableInputs::new(&[(DEFINES, &table)]);
        for node in PROBES {
            ran(plan, node, &mut buffers, &inputs, &mut resources)?;
            match buffers.rows(node) {
                NodeRows::Materialized(rows) => tables.push(rows.clone()),
                NodeRows::Absent => return Err(PortableOperatorFault::NotRun),
            }
            if node == PROBES[0] {
                buffers.release(node);
            }
        }
        for node in (0..5).map(PlanNodeId) {
            buffers.release(node);
        }
    }
    Ok((tables, buffers.builds))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let plan = plan_of(&steps())?;
    Ok(match scenario {
        Scenario::Builds => Observation::Builds(answers(&plan, &FILES[..1])?.1),
        Scenario::BuildsUnshared => Observation::Builds(answers(&unshared(plan), &FILES[..1])?.1),
        Scenario::SharedAgainstOwn => {
            Observation::Same(answers(&plan, &FILES)?.0 == answers(&unshared(plan), &FILES)?.0)
        }
        Scenario::SharedAcrossRows => {
            // Its rows read through the shared index of other rows: a fault or other rows, not the answer of its own.
            let reread = reread(plan);
            let rows = |plan: &PhysicalPlan| answers(plan, &FILES).map(|(rows, _)| rows);
            Observation::Same(rows(&reread) == rows(&unshared(reread)))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
