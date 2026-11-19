//! An equality of a column of each input of a join, both nodes, numbers or flags, is a key of the join: the join meets only the
//! rows the equality keeps, instead of a filter after it.

use super::super::service::push_selections;
use super::{at, relation, same, select, step, tested};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, Side, StepId,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A filter of a join keyed on the file tests a node of the left equal to a node of the right.
    CrossNodeEqualityKeysTheJoin,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_equality_of_a_node_of_each_input_is_a_key_of_the_join"),
    word: Word::Succeeds,
    input: || Scenario::CrossNodeEqualityKeysTheJoin,
    expect: Expect::Value(true),
}];

/// The join of `left` on `left_on` with `right` on `right_on`, each a list of keys, answering `output`.
fn keyed_join(
    (left, left_on): (Reads, &[u32]),
    (right, right_on): (Reads, &[u32]),
    output: &[(Side, u32)],
) -> PlanStep {
    let columns = |keys: &[u32]| keys.iter().copied().map(ColumnIndex).collect();
    step(Operation::Join {
        left,
        right,
        left_on: columns(left_on),
        right_on: columns(right_on),
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
    })
}

/// The plan a row gives, the step it emits, the plan it expects back, and the step that emits there.
type Planned = (Vec<PlanStep>, StepId, Vec<PlanStep>, StepId);

fn placed(scenario: Scenario) -> Planned {
    match scenario {
        Scenario::CrossNodeEqualityKeysTheJoin => {
            // The parent of an edge and an element's id, joined on their file, answering the parent, the id and the kind.
            let answer = &[(Side::Left, 0), (Side::Right, 0), (Side::Right, 1)];
            (
                vec![
                    keyed_join(
                        (relation(Relation::SemanticChild), &[4]),
                        (relation(Relation::SemanticSyntax), &[3]),
                        answer,
                    ),
                    select(at(0), vec![same(0, 1)]),
                ],
                StepId(1),
                vec![keyed_join(
                    (relation(Relation::SemanticChild), &[4, 0]),
                    (relation(Relation::SemanticSyntax), &[3, 0]),
                    answer,
                )],
                StepId(0),
            )
        }
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(tested(placed(scenario), push_selections))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
