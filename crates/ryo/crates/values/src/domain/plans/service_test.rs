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
/// How many inputs are read, or whether the answer was right.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_reads_two_inputs"),
        word: Word::Succeeds,
        input: || Scenario::UnionReads,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_project_reads_one_input"),
        word: Word::Succeeds,
        input: || Scenario::ProjectReads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_hosts_rows_read_nothing"),
        word: Word::Succeeds,
        input: || Scenario::HostedReads,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = PlanInput::Relation(Relation::Function);
    match scenario {
        Scenario::UnionReads => Ok(named(&inputs_of(&Operation::Union {
            left: relation,
            right: PlanInput::Step(StepId(0)),
        }))
        .len()),
        Scenario::ProjectReads => Ok(named(&inputs_of(&Operation::Project {
            from: relation,
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }))
        .len()),
        Scenario::HostedReads => Ok(named(&inputs_of(&Operation::Hosted {
            relation: HostedNumber(0),
        }))
        .len()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
