//! What the `inapplicable_identity` use case promises, against the fakes of the ports it calls, from its fixtures.
//! A restored bound observation answers as a cold run does when the rules of a request skip different files, in every store;
//! a rows request after a findings run is the recorded whole-evaluation decision: it reuses nothing and answers identical rows.
//! Each row states, run by run, whether the answer agreed with a store that keeps nothing.
use super::fixtures::{Answer, Kept, Scenario, Skipping, Then, Warm, observe};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::lint::errors::LintError;
type Input = Scenario;
type Value = Answer;
type Failure = LintError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("consumer_skipping_memory_findings_reuse_the_restored_member"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Consumer,
            kept: Kept::Memory,
            then: Then::Findings,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: true,
            warm: Warm::Reused,
        }),
    },
    Case {
        name: CaseName("consumer_skipping_postcard_findings_reuse_the_restored_member"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Consumer,
            kept: Kept::Postcard,
            then: Then::Findings,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: true,
            warm: Warm::Reused,
        }),
    },
    Case {
        name: CaseName("producer_skipping_memory_findings_reuse_the_restored_member"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Producer,
            kept: Kept::Memory,
            then: Then::Findings,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: false,
            warm: Warm::Reused,
        }),
    },
    Case {
        name: CaseName("producer_skipping_postcard_findings_reuse_the_restored_member"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Producer,
            kept: Kept::Postcard,
            then: Then::Findings,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: false,
            warm: Warm::Reused,
        }),
    },
    Case {
        name: CaseName("rows_after_findings_in_memory_evaluate_again_and_answer_identical_rows"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Consumer,
            kept: Kept::Memory,
            then: Then::Rows,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: true,
            warm: Warm::Evaluated,
        }),
    },
    Case {
        name: CaseName("rows_after_findings_in_postcard_evaluate_again_and_answer_identical_rows"),
        word: Word::Succeeds,
        input: || Scenario {
            skipping: Skipping::Consumer,
            kept: Kept::Postcard,
            then: Then::Rows,
        },
        expect: Expect::Value(Answer {
            cold_rows: true,
            cold_undecided: true,
            cold_skipped: true,
            warm_rows: true,
            warm_undecided: true,
            warm_skipped: true,
            nothing_undecided_warm: true,
            warm: Warm::Evaluated,
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    observe(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
