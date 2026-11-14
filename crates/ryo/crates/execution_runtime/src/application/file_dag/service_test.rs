//! The request's operations prepared once, through `prepare`: one contract for each compiled operation of each rule.

use super::service::prepare;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// The operations of the one rule prepared.
type Input = fn() -> Vec<Operation>;
/// How many contracts the rule's operations prepared.
type Value = usize;
type Failure = OperatingError;

/// A select of the functions' first column.
fn select() -> Operation {
    Operation::Select {
        from: PlanInput::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }
}

/// A projection of `column` of the select before it.
fn project(column: u32) -> Operation {
    Operation::Project {
        from: PlanInput::Step(StepId(0)),
        columns: vec![ColumnIndex(column)],
        distinct: RepeatedRows::Kept,
    }
}

fn subject(operations: Input) -> Result<Value, Failure> {
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: operations()
            .into_iter()
            .map(|operation| PlanStep {
                scope: StepScope::PerFile,
                operation,
            })
            .collect(),
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let operators = prepare(
        &[plan],
        &[vec![]],
        &[],
        std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )?;
    Ok(operators.rules.iter().map(Vec::len).sum())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_prepares_one_contract"),
        word: Word::Succeeds,
        input: || || vec![select()],
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_select_and_its_projection_prepare_one_contract_each"),
        word: Word::Succeeds,
        input: || || vec![select(), project(0)],
        expect: Expect::Value(2),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
