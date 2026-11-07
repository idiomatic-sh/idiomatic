//! Both authored union associations must publish the same exact distinct findings.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{HostedNumber, Input as PlanInput, StepId},
        plans::types::Operation,
    },
};
type Input = ();
type Value = bool;
type Failure = OperatingError;
fn subject((): Input) -> Result<Value, Failure> {
    let hosts = vec![
        fixtures::requests::numbers("a", 1, &[&[1], &[2]]),
        fixtures::requests::numbers("b", 1, &[&[1], &[2]]),
        fixtures::requests::numbers("c", 1, &[&[2], &[3]]),
        fixtures::requests::numbers("d", 1, &[&[1], &[3]]),
    ];
    let source_steps: Vec<_> = (0..4)
        .map(|index| Operation::Hosted {
            relation: HostedNumber(index),
        })
        .collect();
    let mut balanced = source_steps.clone();
    balanced.extend([
        Operation::Union {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
        },
        Operation::Union {
            left: PlanInput::Step(StepId(2)),
            right: PlanInput::Step(StepId(3)),
        },
        Operation::Union {
            left: PlanInput::Step(StepId(4)),
            right: PlanInput::Step(StepId(5)),
        },
    ]);
    let mut left_deep = source_steps;
    left_deep.extend([
        Operation::Union {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
        },
        Operation::Union {
            left: PlanInput::Step(StepId(4)),
            right: PlanInput::Step(StepId(2)),
        },
        Operation::Union {
            left: PlanInput::Step(StepId(5)),
            right: PlanInput::Step(StepId(3)),
        },
    ]);
    let balanced = fixtures::requests::published(
        inputs::plan(balanced),
        vec![],
        hosts.clone(),
        &mut Symbols::default(),
    )?;
    let left_deep = fixtures::requests::published(
        inputs::plan(left_deep),
        vec![],
        hosts,
        &mut Symbols::default(),
    )?;
    assert_eq!(balanced.count, left_deep.count);
    assert_eq!(balanced.cells, left_deep.cells);
    assert_eq!(
        balanced.cells,
        [
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3))
        ]
    );
    assert_eq!(balanced.count.0, 3);
    Ok(true)
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("repeated_or_alternatives_are_one_finding_in_balanced_or_left_deep_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
