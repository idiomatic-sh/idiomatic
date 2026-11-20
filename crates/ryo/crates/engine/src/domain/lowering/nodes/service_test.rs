//! The cases of the `nodes` part: the pack's store of merged plan nodes. A node is one digest: two steps that differ in one
//! literal are two nodes, and a sub-plan two rules hold is one node with a reader in each.

use super::service::{digest_of, readers, stored};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::named;
use crate::domain::lowering::tables::service::admitted;
use crate::domain::lowering::types::SelectedRule;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The scenes whose plans are stored together, one rule each.
type Input = &'static [&'static str];
/// The nodes of the store; the readers of the first rule's step 0; the readers of the first rule's emitted step.
type Value = (usize, usize, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_rule_of_two_steps_is_two_nodes_each_read_once"),
        word: Word::Succeeds,
        input: || &["the words a"],
        expect: Expect::Value((2, 1, 1)),
    },
    Case {
        name: CaseName("two_steps_that_differ_in_one_literal_are_two_nodes_and_the_step_both_read_is_one_with_two_readers"),
        word: Word::Succeeds,
        input: || &["the words a", "the words b"],
        expect: Expect::Value((3, 2, 1)),
    },
    Case {
        name: CaseName("one_plan_selected_twice_is_the_same_nodes_and_its_emitted_step_has_a_reader_in_each_rule"),
        word: Word::Succeeds,
        input: || &["the words a", "the words a"],
        expect: Expect::Value((2, 1, 2)),
    },
];

fn subject(scenes: Input) -> Result<Value, Failure> {
    let plans: Vec<_> = scenes.iter().map(|scene| named(scene)).collect();
    let mut rules = Vec::new();
    for (plan, ordinal) in plans.iter().zip(0..) {
        let rule = SelectedRule {
            rule: SelectedRuleOrdinal(ordinal),
            plan,
            bindings: RuleBindings(&[]),
        };
        rules.push(admitted(rule).map_err(|_| ())?);
    }
    let store = stored(&rules);
    let first = rules.first().ok_or(())?;
    let emitted = first.rule.plan.emit.step;
    Ok((
        store.0.len(),
        readers(&store, digest_of(first, StepId(0))),
        readers(&store, digest_of(first, emitted)),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
