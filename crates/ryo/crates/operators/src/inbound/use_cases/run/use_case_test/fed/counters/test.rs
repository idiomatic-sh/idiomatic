//! The node counters, through `plan` and `run` only: over two files, each node made once per file, its rows read and written
//! summed, a node that wrote no row counted as an empty making, and nothing counted when counting is off.
use super::super::super::super::fixtures::cases::{DEFINES, defines, named, select};
use super::super::super::super::fixtures::relational::{TEXTS, node, project, relation};
use super::super::super::super::fixtures::{TableInputs, TextResources, planned, ran};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{LogicalNode, NodeBindings, PlanInputList, PlanNodeId};
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepScope};
use std::borrow::Cow;

/// Whether the run counts its nodes.
#[derive(Clone, Copy, Debug)]
enum Counting {
    /// Counting is on.
    On,
    /// The planted counter-example: counting is off.
    Off,
}
type Input = Counting;
/// One line per node made: its id, kind, makings, makings that wrote no row, rows read and rows written, and how much of its
/// time is that of the makings that wrote no row: all, none or part.
type Value = Cow<'static, str>;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("counters_sum_each_nodes_makings_and_rows_over_the_files"),
        word: Word::Succeeds,
        input: || Counting::On,
        // Each file holds three `DEFINES` rows, two named `a`: the select of `a` writes 2 of 3, the projection and the select
        // after it 2 of 2, the select of `zzz` none of 2; each node made once in each of the two files.
        expect: Expect::Value(Cow::Borrowed(COUNTED)),
    },
    Case {
        name: CaseName("counters_off_count_nothing"),
        word: Word::Succeeds,
        input: || Counting::Off,
        expect: Expect::Value(Cow::Borrowed("off")),
    },
];

/// What `counters_sum_each_nodes_makings_and_rows_over_the_files` reads off a run.
const COUNTED: &str = "0: Some(Filter) runs 2 empty 0 in 6 out 4 empty time none\n\
                       1: Some(Project) runs 2 empty 0 in 4 out 4 empty time none\n\
                       2: Some(Filter) runs 2 empty 0 in 4 out 4 empty time none\n\
                       3: Some(Filter) runs 2 empty 2 in 4 out 0 empty time all";

/// The program: a select of `a` read by a projection, a select of the projection, and a select of `zzz`; the last two emitted.
fn steps() -> Vec<Step> {
    vec![
        select(relation(), vec![named("a")]),
        project(node(0), &[3], RepeatedRows::Kept),
        select(node(1), Vec::new()),
        select(node(0), vec![named("zzz")]),
    ]
}

/// Runs the program over two files on one worker's buffers, counting when `counting` says, and answers the counts.
fn counted(counting: Counting) -> Result<String, Failure> {
    let steps = steps();
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    let emits = [PlanNodeId(2), PlanNodeId(3)];
    for (rule, node) in (0..).map(RuleOrdinal).zip(&emits) {
        program.emit(rule, *node);
    }
    let plan = planned(&program)?;
    let mut texts = TextResources::new(TEXTS);
    let rows = defines(&[("r1", "a"), ("r2", "b"), ("r1", "a")], &texts);
    let inputs = TableInputs::new(&[(DEFINES, &rows)]);
    let mut buffers = FileBuffers::default();
    if let Counting::On = counting {
        buffers.count_nodes();
    }
    for _file in 0..2 {
        for node in emits {
            ran(&plan, node, &mut buffers, &inputs, &mut texts)?;
        }
        for node in (0..4).map(PlanNodeId) {
            buffers.release(node);
        }
    }
    let Some(counters) = buffers.take_counters() else {
        return Ok("off".to_owned());
    };
    Ok(counters
        .counts()
        .map(|counted| {
            let work = counted.work;
            format!(
                "{}: {:?} runs {} empty {} in {} out {} empty time {}",
                counted.node.0,
                plan.node_kind(counted.node),
                work.runs.0,
                work.empty_runs.0,
                work.rows_in.0,
                work.rows_out.0,
                if work.empty_time.is_zero() {
                    "none"
                } else if work.empty_time == work.time {
                    "all"
                } else {
                    "part"
                },
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn subject(counting: Input) -> Result<Value, Failure> {
    counted(counting).map(Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
