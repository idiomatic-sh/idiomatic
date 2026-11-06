//! The rows a file hands over are cut to the columns the steps after the files ask, and no further.

use super::service::cut_relations;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A step after the files asks of one column of a relation.
    OneColumnAsked,
    /// A step after the files asks of every column of a relation.
    EveryColumnAsked,
}

/// Whether the plan came back with a step that cuts the relation.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_asked_for_one_column_is_cut_to_it"),
        word: Word::Succeeds,
        input: || Scenario::OneColumnAsked,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_relation_asked_for_every_column_is_left_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::EveryColumnAsked,
        expect: Expect::Value(false),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kept = match scenario {
        Scenario::OneColumnAsked => vec![ColumnIndex(0)],
        Scenario::EveryColumnAsked => Vec::new(),
    };
    let steps = vec![PlanStep {
        operation: Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept,
        },
        scope: StepScope::CrossFile,
    }];
    let planned = cut_relations(steps.clone(), StepId(0));
    Ok(planned.steps.len() > steps.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
