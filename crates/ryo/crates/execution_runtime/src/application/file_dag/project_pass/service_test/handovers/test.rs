//! What a phase holds of each rule's handover, through `run_batch`: a project relation handed to many rules is the phase's one
//! table, each rule's frontier is proven before any occurrence evaluates and decoded at its first occurrence that does, and its
//! handover and node rows go after its last occurrence.
use super::fixtures::{FIRST, Ran, ran};
use crate::application::file_dag::project_pass::service::uses_of;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// The phase's declared occurrences, and what of the batch a case reads.
#[derive(Clone, Copy, Debug)]
enum Phase {
    /// Rules 0 and 1, each counted as occurring once more after the batch: how many hold the project relation's table.
    Shared,
    /// Rules 0 then 1: the store reads and the guard, in order.
    Order,
    /// Rules 0 then 1: whether each rule's emit still holds rows after the batch, and each outcome.
    Released,
    /// Rules 0, 1, then 0 again: the store reads and the guard, in order, and each outcome.
    Twice,
    /// Rules 0 then 1, the store refusing rule 1's file frontier: each outcome, and the store reads.
    Refused,
}

type Input = Phase;
/// What the batch left, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_project_relation_handed_to_many_rules_is_held_once"),
        word: Word::Succeeds,
        input: || Phase::Shared,
        expect: Expect::Value(Cow::Borrowed("holders 3")),
    },
    Case {
        name: CaseName("a_rule_is_decoded_at_its_first_occurrence"),
        word: Word::Succeeds,
        input: || Phase::Order,
        expect: Expect::Value(Cow::Borrowed(
            "[\"kept 0\", \"kept 16\", \"guard 0\", \"kept 0\", \"guard 1\", \"kept 16\"]",
        )),
    },
    Case {
        name: CaseName("a_rule_handover_goes_after_its_last_occurrence"),
        word: Word::Succeeds,
        input: || Phase::Released,
        expect: Expect::Value(Cow::Borrowed(
            "emits [\"absent\", \"absent\"]; [\"complete RowCount(1)\", \"complete RowCount(1)\"]",
        )),
    },
    Case {
        name: CaseName("a_rule_occurring_twice_is_decoded_once"),
        word: Word::Succeeds,
        input: || Phase::Twice,
        expect: Expect::Value(Cow::Borrowed(
            "[\"kept 0\", \"kept 16\", \"guard 0\", \"kept 0\", \"guard 1\", \"kept 16\"]; [\"complete RowCount(1)\", \"complete RowCount(1)\", \"complete RowCount(1)\"]",
        )),
    },
    Case {
        name: CaseName("a_store_refusal_stops_its_occurrence_before_any_evaluates"),
        word: Word::Succeeds,
        input: || Phase::Refused,
        expect: Expect::Value(Cow::Borrowed(
            "[\"complete RowCount(1)\", \"stopped frontiers the facts store failed: the kept frontier is unreadable\"]; [\"kept 0\", \"kept 16\", \"guard 0\", \"kept 0\"]",
        )),
    },
];

fn subject(phase: Input) -> Result<Value, Failure> {
    let both = [RuleIndex(0), RuleIndex(1)];
    let text = match phase {
        Phase::Shared => {
            let counted = uses_of(both.iter().chain(&both).copied());
            let Ran { holders, .. } = ran(&both, Some(counted), None);
            format!("holders {holders}")
        }
        Phase::Order => format!("{:?}", ran(&both, None, None).log),
        Phase::Released => {
            let Ran {
                emits, outcomes, ..
            } = ran(&both, None, None);
            format!("emits {emits:?}; {outcomes:?}")
        }
        Phase::Twice => {
            let Ran { log, outcomes, .. } =
                ran(&[RuleIndex(0), RuleIndex(1), RuleIndex(0)], None, None);
            format!("{log:?}; {outcomes:?}")
        }
        Phase::Refused => {
            let Ran { log, outcomes, .. } = ran(&both, None, Some(FIRST[1]));
            format!("{outcomes:?}; {log:?}")
        }
    };
    Ok(Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
