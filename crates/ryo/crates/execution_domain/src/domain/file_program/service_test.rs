//! The request graph: one node per distinct operation over the same inputs, shared by every rule that makes it.
use super::service::compile_prepared;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// The plans of a request.
#[derive(Clone, Copy)]
enum Request {
    /// Two rules whose first step selects the functions, the second rule selecting the comments next.
    SharedFirstStep,
}
type Input = Request;
/// The node of each step of each rule, in rule then step order, then how many nodes the graph holds.
type Value = [usize; 4];
type Failure = OperatingError;

fn select(from: PlanInput) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from,
            comparisons: vec![],
            kept: vec![],
        },
    }
}
fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        frontier: vec![],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len() - 1).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
    }
}

/// The graph the request's plans compile to.
fn subject(request: Input) -> Result<Value, Failure> {
    let plans = match request {
        Request::SharedFirstStep => vec![
            plan(vec![select(PlanInput::Relation(Relation::Function))]),
            plan(vec![
                select(PlanInput::Relation(Relation::Function)),
                select(PlanInput::Relation(Relation::Comment)),
            ]),
        ],
    };
    let operators = super::service::operators::service::prepare(
        &plans,
        &vec![vec![]; plans.len()],
        &[],
        &ryo_relations::domain::relations::service::kinds,
        &mut |_| Ok(()),
    )?;
    let graph = compile_prepared(&plans, &operators);
    let mut answer = graph
        .program_nodes
        .iter()
        .flatten()
        .map(|node| node.as_ref().map(|node| node.0).map_err(Clone::clone))
        .collect::<Result<Vec<_>, _>>()?;
    answer.push(graph.nodes.len());
    answer.try_into().map_err(|_| OperatingError::ShapeMismatch)
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("rules_making_the_same_first_step_share_its_node"),
    word: Word::Succeeds,
    input: || Request::SharedFirstStep,
    expect: Expect::Value([0, 0, 1, 2]),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
