//! What a phase holds of each rule's handover, through `run_batch`: a project relation handed to many rules is the phase's one
//! table; each rule's frontier is proven from its provenance before any occurrence evaluates, and its words are read once, at
//! its first occurrence that evaluates, carrying the coverage of the proof read with them; and its handover and node rows go
//! after its last occurrence, before the next rule evaluates.
use super::fixtures::{Ran, ran};
use crate::application::file_dag::project_pass::service::uses_of;
use crate::outbound::facts::fixtures::project_phase_handovers::{FIRST_BUCKETS, Store};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// The phase's declared occurrences, the store, and what of the batch a case reads.
#[derive(Clone, Copy, Debug)]
enum Phase {
    /// Rules 0 and 1, each counted as occurring once more after the batch: how many hold the project relation's table.
    Shared,
    /// Rules 0 then 1: the store reads and the guard, in order.
    Order,
    /// Rules 0 then 1: the log, then where each handover stands, what each rule's nodes hold and the table's holders after.
    Released,
    /// Rules 0, 1, then 0 again: the same, and each outcome.
    Twice,
    /// Rules 0 then 1, the store refusing rule 1's file frontier: each outcome, and the store reads.
    Refused,
    /// Rule 0, counted once more after the batch, its file frontier complete when proven and partial when decoded: where its
    /// handover stands after the batch.
    CoverageChanged,
    /// Rule 0, its file frontier proven empty and decoded as a file the run does not have: its outcome, and the store reads.
    OutsiderOnDecode,
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
            "[\"sources 0\", \"sources 16\", \"guard 0 holders 3\", \"kept 0\", \"guard 1 holders 2\", \"kept 16\"]",
        )),
    },
    Case {
        name: CaseName("a_rule_handover_goes_after_its_last_occurrence"),
        word: Word::Succeeds,
        input: || Phase::Released,
        expect: Expect::Value(Cow::Borrowed(
            "[\"sources 0\", \"sources 16\", \"guard 0 holders 3\", \"kept 0\", \"guard 1 holders 2\", \"kept 16\"]; [\"rule 0 released\", \"rule 1 released\"]; [\"rule 0 holds 0\", \"rule 1 holds 0\"]; holders 1",
        )),
    },
    Case {
        name: CaseName("a_rule_occurring_twice_is_decoded_once"),
        word: Word::Succeeds,
        input: || Phase::Twice,
        expect: Expect::Value(Cow::Borrowed(
            "[\"sources 0\", \"sources 16\", \"guard 0 holders 3\", \"kept 0\", \"guard 1 holders 3\", \"kept 16\"]; [\"rule 0 released\", \"rule 1 released\"]; [\"rule 0 holds 0\", \"rule 1 holds 0\"]; holders 1; [\"complete RowCount(1)\", \"complete RowCount(1)\", \"complete RowCount(1)\"]",
        )),
    },
    Case {
        name: CaseName("a_store_refusal_stops_its_occurrence_before_any_evaluates"),
        word: Word::Succeeds,
        input: || Phase::Refused,
        expect: Expect::Value(Cow::Borrowed(
            "[\"complete RowCount(1)\", \"stopped frontiers the facts store failed: the kept frontier is unreadable\"]; [\"sources 0\", \"sources 16\", \"guard 0 holders 2\", \"kept 0\"]",
        )),
    },
    Case {
        name: CaseName("decoded_words_carry_the_coverage_of_their_own_proof"),
        word: Word::Succeeds,
        input: || Phase::CoverageChanged,
        expect: Expect::Value(Cow::Borrowed(
            "[\"rule 0 handed Some(Partial { files: [Some(FileId(0))], lacks: [] })\"]",
        )),
    },
    Case {
        name: CaseName("decoded_words_whose_own_provenance_does_not_fit_are_refused"),
        word: Word::Succeeds,
        input: || Phase::OutsiderOnDecode,
        expect: Expect::Value(Cow::Borrowed(
            "[\"stopped frontiers named producer is incomplete: frontier source proof does not match the prepared program\"]; [\"sources 0\", \"guard 0 holders 2\", \"kept 0\"]",
        )),
    },
];

fn subject(phase: Input) -> Result<Value, Failure> {
    let both = [RuleIndex(0), RuleIndex(1)];
    let after = |ran: &Ran| {
        format!(
            "{:?}; {:?}; {:?}; holders {}",
            ran.log, ran.handovers, ran.buffers, ran.holders
        )
    };
    let text = match phase {
        Phase::Shared => {
            let counted = uses_of(both.iter().chain(&both).copied());
            let Ran { holders, .. } = ran(&both, Some(counted), Store::Empty);
            format!("holders {holders}")
        }
        Phase::Order => format!("{:?}", ran(&both, None, Store::Empty).log),
        Phase::Released => after(&ran(&both, None, Store::Empty)),
        Phase::Twice => {
            let twice = ran(
                &[RuleIndex(0), RuleIndex(1), RuleIndex(0)],
                None,
                Store::Empty,
            );
            format!("{}; {:?}", after(&twice), twice.outcomes)
        }
        Phase::Refused => {
            let Ran { log, outcomes, .. } = ran(&both, None, Store::Refusing(FIRST_BUCKETS[1]));
            format!("{outcomes:?}; {log:?}")
        }
        Phase::CoverageChanged => {
            let rule = [RuleIndex(0)];
            let counted = uses_of(rule.iter().chain(&rule).copied());
            let Ran { handovers, .. } = ran(&rule, Some(counted), Store::CoverageChanged);
            format!("{handovers:?}")
        }
        Phase::OutsiderOnDecode => {
            let Ran { log, outcomes, .. } = ran(&[RuleIndex(0)], None, Store::OutsiderOnDecode);
            format!("{outcomes:?}; {log:?}")
        }
    };
    Ok(Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
