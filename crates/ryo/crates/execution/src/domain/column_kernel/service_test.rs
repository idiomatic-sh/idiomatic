//! The `column_kernel` service's cases, on values alone.

use crate::domain::column_kernel::service::compile;
use crate::domain::operator::types::PlanLayout;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Source, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

#[derive(Clone, Copy)]
enum Scenario {
    NoSteps,
    SelectOnly,
}
type Input = Scenario;
type Value = ();
type Failure = ();

/// A plan of the given steps that emits its last one.
fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier: vec![],
        comparison_origins: vec![],
    }
}

fn subject(scenario: Input) {
    let steps = match scenario {
        Scenario::NoSteps => vec![],
        Scenario::SelectOnly => vec![PlanStep {
            operation: Operation::Select {
                from: Source::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![],
            },
            scope: StepScope::PerFile,
        }],
    };
    let expected = steps.len();
    let chains = compile(&plan(steps), &PlanLayout::default());
    assert_eq!(chains.0.len(), expected);
    assert!(chains.0.iter().all(Option::is_none));
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_plan_without_steps_has_no_chain"),
        word: Word::Succeeds,
        input: || Scenario::NoSteps,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_plan_that_only_selects_has_no_chain_at_its_step"),
        word: Word::Succeeds,
        input: || Scenario::SelectOnly,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        subject(input);
        Ok(())
    });
}
