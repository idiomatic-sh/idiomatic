//! Planning grows with the program, not with its square: a program of four hundred rules plans in about four times the time of
//! one of a hundred. Every node's readers, and each probe's shared index, are found once for the plan.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::service::plan;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{LogicalNode, NodeBindings, PlanInputList, PlanNodeId};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::time::Duration;

/// The program sizes compared: a hundred rules, then four hundred.
#[derive(Clone, Copy, Debug)]
enum Sizes {
    HundredThenFourHundred,
}
type Input = Sizes;
type Value = Growth;
type Failure = PortableOperatorFault;

/// How planning time grows from the smaller program to the larger.
#[derive(Debug, PartialEq, Eq)]
enum Growth {
    /// Within twice the growth of the programs: four times the rules plan in under eight times the time.
    Linear,
    /// Past it: the square of four is sixteen.
    Superlinear,
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("planning_four_hundred_rules_grows_linearly"),
    word: Word::Succeeds,
    input: || Sizes::HundredThenFourHundred,
    expect: Expect::Value(Growth::Linear),
}];

/// The three nodes of one rule from node `at`: a select of Defines, a select of it, and a join probing the first by the
/// second, so every rule has a streaming stage and a probe index of its own.
fn rule(at: u32) -> [Step; 3] {
    let select = |from| Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    [
        over_defines(select(StepInput::Relation(Relation::Defines))),
        over_nodes(select(StepInput::Step(StepId(at))), &[at]),
        over_nodes(
            Operation::Join {
                left: StepInput::Step(StepId(at + 1)),
                right: StepInput::Step(StepId(at)),
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
            &[at + 1, at],
        ),
    ]
}

/// The fastest of three plans of a program of `rules` rules, each emitting its join.
fn planned(rules: u32) -> Result<Duration, Failure> {
    let steps: Vec<Step> = (0..rules).flat_map(|each| rule(each * 3)).collect();
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for each in 0..rules {
        program.emit(RuleOrdinal(each), PlanNodeId(each * 3 + 2));
    }
    let mut fastest = Duration::MAX;
    for _ in 0..3 {
        let started = std::time::Instant::now();
        let _plan = plan(&program)?;
        fastest = fastest.min(started.elapsed());
    }
    Ok(fastest)
}

fn subject(sizes: Input) -> Result<Value, Failure> {
    let (small, large) = match sizes {
        Sizes::HundredThenFourHundred => (planned(100)?, planned(400)?),
    };
    Ok(if large < small.saturating_mul(8) {
        Growth::Linear
    } else {
        Growth::Superlinear
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
