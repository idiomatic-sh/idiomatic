//! The `prepared_request` service's cases: plans, bindings and rules are assembled only when each plan has its rule and its bindings.

use super::service::assemble;
use crate::domain::rule_intent::errors::RuleIntentError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases gives: how many lists of bindings, for no plans and no rules.
type Input = usize;
/// How many plans the assembled request holds.
type Value = usize;
/// What the assembly refuses with.
type Failure = RuleIntentError;

fn subject(bindings: Input) -> Result<Value, Failure> {
    assemble(Vec::new(), vec![Vec::new(); bindings], &[]).map(|compiled| compiled.plans.len())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rules_assemble_to_no_plans"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("bindings_without_a_rule_are_an_invalid_reference"),
        word: Word::Refuses,
        input: || 1,
        expect: Expect::Error(RuleIntentError::InvalidReference),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
