//! Refused calls are not deliveries; capture is owed; release is keyed by relation.
use super::fixtures::{Answer, Authoring, Seen, Step, Trace, script};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

/// The program a row opens one file of, and the steps it takes on it.
#[derive(Clone, Copy, Debug)]
struct Script {
    authoring: Authoring,
    steps: &'static [Step],
}

type Input = Script;
type Value = Trace;
type Failure = OperatingError;

/// A step that answered this, leaving these batches retained after these walk passes.
const fn seen(answer: Answer, retained: usize, passes: u64) -> Seen {
    Seen {
        answer,
        retained,
        passes,
    }
}

/// The trace of a file that took these steps and finished without a frontier readiness.
const fn trace(steps: &'static [Seen]) -> Trace {
    Trace {
        steps: Cow::Borrowed(steps),
        readiness: false,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_rule_owing_rows_and_capture_keeps_the_batch_until_its_capture_is_delivered",
        ),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Captured,
            steps: const {
                &[
                    Step::Observe(RuleIndex(0)),
                    Step::Observe(RuleIndex(1)),
                    Step::Capture(RuleIndex(0)),
                    Step::Capture(RuleIndex(1)),
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::Rows(2), 1, 1),
                    seen(Answer::Rows(2), 1, 1),
                    seen(Answer::Captured(0), 1, 1),
                    seen(Answer::Captured(0), 0, 1),
                ]
            },
        )),
    },
    Case {
        name: CaseName(
            "a_refused_entry_point_call_does_not_consume_the_delivery_it_would_have_been",
        ),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Continuations,
            steps: const {
                &[
                    Step::Deliver(RuleIndex(2)),
                    Step::Rows {
                        rule: RuleIndex(3),
                        refuse: false,
                    },
                    Step::Capture(RuleIndex(3)),
                    Step::Observe(RuleIndex(3)),
                    Step::Deliver(RuleIndex(1)),
                    Step::Deliver(RuleIndex(0)),
                    Step::Rows {
                        rule: RuleIndex(2),
                        refuse: false,
                    },
                    Step::Deliver(RuleIndex(3)),
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::Refused(OperatingError::NotRun), 0, 0),
                    seen(Answer::Refused(OperatingError::NotRun), 0, 0),
                    seen(Answer::Refused(OperatingError::NotRun), 0, 0),
                    seen(Answer::Refused(OperatingError::NotRun), 0, 0),
                    seen(Answer::Rows(2), 1, 1),
                    seen(Answer::Failed(OperatingError::WrongOperation), 1, 1),
                    seen(Answer::Emitted { complete: true }, 1, 1),
                    seen(Answer::Rows(2), 0, 1),
                ]
            },
        )),
    },
    Case {
        name: CaseName(
            "retrying_a_cancelled_forwarded_delivery_after_retirement_finds_no_batch_and_walks_nothing",
        ),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Continuations,
            steps: const {
                &[
                    Step::Deliver(RuleIndex(1)),
                    Step::Deliver(RuleIndex(3)),
                    Step::Deliver(RuleIndex(0)),
                    Step::Rows {
                        rule: RuleIndex(2),
                        refuse: true,
                    },
                    Step::Rows {
                        rule: RuleIndex(2),
                        refuse: false,
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::Rows(2), 1, 1),
                    seen(Answer::Rows(2), 1, 1),
                    seen(Answer::Failed(OperatingError::WrongOperation), 1, 1),
                    seen(Answer::Refused(OperatingError::NotRead), 0, 1),
                    seen(Answer::Emitted { complete: true }, 0, 1),
                ]
            },
        )),
    },
    Case {
        name: CaseName("a_relation_is_released_while_another_relation_is_still_owed"),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Relations,
            steps: const {
                &[
                    Step::SameClass(RuleIndex(0), RuleIndex(1)),
                    Step::SameClass(RuleIndex(2), RuleIndex(3)),
                    Step::Deliver(RuleIndex(0)),
                    Step::Deliver(RuleIndex(2)),
                    Step::Deliver(RuleIndex(1)),
                    Step::Deliver(RuleIndex(3)),
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::SameClass(false), 0, 0),
                    seen(Answer::SameClass(true), 0, 0),
                    seen(Answer::Rows(2), 2, 2),
                    seen(Answer::Rows(3), 2, 2),
                    seen(Answer::Rows(1), 1, 2),
                    seen(Answer::Rows(3), 0, 2),
                ]
            },
        )),
    },
    Case {
        name: CaseName("a_batch_stays_while_a_rule_of_another_class_on_its_relation_still_owes"),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Relations,
            steps: const {
                &[
                    Step::Deliver(RuleIndex(0)),
                    Step::Deliver(RuleIndex(1)),
                    Step::Deliver(RuleIndex(2)),
                    Step::Deliver(RuleIndex(3)),
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::Rows(2), 2, 2),
                    seen(Answer::Rows(1), 1, 2),
                    seen(Answer::Rows(3), 1, 2),
                    seen(Answer::Rows(3), 0, 2),
                ]
            },
        )),
    },
    Case {
        name: CaseName("a_batch_stays_while_a_later_class_on_its_relation_still_owes"),
        word: Word::Succeeds,
        input: || Script {
            authoring: Authoring::Relations,
            steps: const {
                &[
                    Step::Deliver(RuleIndex(1)),
                    Step::Deliver(RuleIndex(0)),
                    Step::Deliver(RuleIndex(2)),
                    Step::Deliver(RuleIndex(3)),
                ]
            },
        },
        expect: Expect::Value(trace(
            const {
                &[
                    seen(Answer::Rows(1), 2, 2),
                    seen(Answer::Rows(2), 1, 2),
                    seen(Answer::Rows(3), 1, 2),
                    seen(Answer::Rows(3), 0, 2),
                ]
            },
        )),
    },
];

/// Opens one file of the script's program and runs its steps over it.
fn subject(input: Input) -> Result<Value, Failure> {
    script(input.authoring, input.steps)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
