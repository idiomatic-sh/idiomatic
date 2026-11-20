//! The `plans` service's cases, on values alone.

use super::service::{inputs_of, named};
use super::types::Operation;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input as PlanInput;
use crate::domain::plan_parts::types::RepeatedRows;
use crate::domain::plan_parts::types::StepId;
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// What a union reads.
    UnionReads,
    /// What a project reads.
    ProjectReads,
    /// What the host's rows read.
    HostedReads,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// How many inputs are read, as text.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_reads_two_inputs"),
        word: Word::Succeeds,
        input: || Scenario::UnionReads,
        expect: Expect::Value(Cow::Borrowed("2")),
    },
    Case {
        name: CaseName("a_project_reads_one_input"),
        word: Word::Succeeds,
        input: || Scenario::ProjectReads,
        expect: Expect::Value(Cow::Borrowed("1")),
    },
    Case {
        name: CaseName("the_hosts_rows_read_nothing"),
        word: Word::Succeeds,
        input: || Scenario::HostedReads,
        expect: Expect::Value(Cow::Borrowed("0")),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = PlanInput::Relation(Relation::Function);
    let count = |operation: &Operation| Cow::Owned(named(&inputs_of(operation)).len().to_string());
    Ok(match scenario {
        Scenario::UnionReads => count(&Operation::Union {
            left: relation,
            right: PlanInput::Step(StepId(0)),
        }),
        Scenario::ProjectReads => count(&Operation::Project {
            from: relation,
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
        Scenario::HostedReads => count(&Operation::Hosted {
            relation: HostedNumber(0),
        }),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
