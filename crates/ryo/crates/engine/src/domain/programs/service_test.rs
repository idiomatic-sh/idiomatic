//! The `programs` service's cases: a rule set with no accepted rule reads no relation and runs no rule.

use super::service::{accepted, columns, needs};
use super::types::Programs;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row reads of an empty rule set.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The relations it reads.
    Needs,
    /// The rules it runs.
    Accepted,
    /// The columns of the rules it runs.
    Columns,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_rule_set_reads_no_relation"),
        word: Word::Succeeds,
        input: || Scenario::Needs,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an_empty_rule_set_runs_no_rule"),
        word: Word::Succeeds,
        input: || Scenario::Accepted,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an_empty_rule_set_emits_no_column"),
        word: Word::Succeeds,
        input: || Scenario::Columns,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let programs = Programs::default();
    Ok(match scenario {
        Scenario::Needs => needs(&programs).0.len(),
        Scenario::Accepted => accepted(&programs).len(),
        Scenario::Columns => columns(&programs).len(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
