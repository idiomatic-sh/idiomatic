//! The `plans` service's cases, on values alone.

use super::service::{inputs_of, named, step_scope};
use super::types::{Emit, Operation, Plan, PlanStep};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input as PlanInput;
use crate::domain::plan_parts::types::RepeatedRows;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::StepScope;
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
    /// Where a step of the plan runs.
    StepRuns,
    /// Where a step the plan lacks runs.
    NoSuchStep,
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
    Case {
        name: CaseName("a_step_of_the_plan_has_its_scope"),
        word: Word::Succeeds,
        input: || Scenario::StepRuns,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_step_the_plan_lacks_has_none"),
        word: Word::Succeeds,
        input: || Scenario::NoSuchStep,
        expect: Expect::Value(1),
    },
];

fn plan_of(operation: Operation) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation,
            scope: StepScope::CrossFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

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
        Scenario::StepRuns => {
            let plan = plan_of(Operation::Hosted {
                relation: HostedNumber(0),
            });
            Ok(usize::from(
                step_scope(&plan, StepId(0)) == Some(StepScope::CrossFile),
            ))
        }
        Scenario::NoSuchStep => {
            let plan = plan_of(Operation::Hosted {
                relation: HostedNumber(0),
            });
            Ok(usize::from(step_scope(&plan, StepId(5)).is_none()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
