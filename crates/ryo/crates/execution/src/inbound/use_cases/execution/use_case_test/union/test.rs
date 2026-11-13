//! Both authored union associations must publish the same exact distinct findings.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{HostedNumber, Input as PlanInput, StepId},
        plans::types::Operation,
    },
};
use std::borrow::Cow;
type Input = ();
/// What the balanced and the left-deep unions published: their row counts, whether their cells are the same, and the
/// balanced union's numbers in order.
#[derive(Debug, PartialEq, Eq)]
struct Unions {
    rows: [usize; 2],
    same_cells: bool,
    numbers: Cow<'static, [i64]>,
}
type Value = Unions;
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
    Ok(Unions {
        rows: [balanced.count.0, left_deep.count.0],
        same_cells: balanced.cells == left_deep.cells,
        numbers: balanced
            .cells
            .iter()
            .filter_map(|cell| {
                if let Cell::Number(number) = cell {
                    Some(number.0)
                } else {
                    None
                }
            })
            .collect(),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("repeated_or_alternatives_are_one_finding_in_balanced_or_left_deep_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Unions {
        rows: [3, 3],
        same_cells: true,
        numbers: Cow::Borrowed(&[1, 2, 3]),
    }),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
