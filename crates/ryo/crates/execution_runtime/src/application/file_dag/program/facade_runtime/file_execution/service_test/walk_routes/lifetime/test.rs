//! A shared source batch lives until its last consumer is delivered, on every exit.
//!
//! The held-lease row leases through the test-only `lease_any` on the batch map, because a
//! `KernelPermit` cannot be built outside the runtime. It proves a held lease stays readable
//! across retirement and that a later demand finds no batch; it does not exercise the permit
//! path of a real nested kernel demand.
use super::super::fixtures::Residency;
use super::fixtures::{Answer, Seen, Step, Trace, script};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

/// The files a row opens over the continuation program, with this residency, and the steps it
/// takes on them.
#[derive(Clone, Copy, Debug)]
struct Script {
    residency: Residency,
    files: usize,
    steps: &'static [Step],
}

type Input = Script;
type Value = Trace;
type Failure = OperatingError;

/// A step that answered this, leaving these batches retained in each open file.
const fn seen(answer: Answer, retained: &'static [usize]) -> Seen {
    Seen {
        answer,
        retained: Cow::Borrowed(retained),
    }
}

/// The trace of files that opened with these batches retained, took these steps, made these walk
/// passes over these source rows, and finished with or without a frontier readiness.
const fn trace(
    opened: &'static [usize],
    steps: &'static [Seen],
    counts: &'static [(u64, u64)],
    readiness: &'static [bool],
) -> Trace {
    Trace {
        opened: Cow::Borrowed(opened),
        steps: Cow::Borrowed(steps),
        counts: Cow::Borrowed(counts),
        readiness: Cow::Borrowed(readiness),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "success_of_the_last_consumer_retires_the_shared_batch_and_keeps_its_accounting",
        ),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Stable,
            files: 1,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: false,
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0],
            const {
                &[
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[1]),
                    seen(Answer::Forwarded { complete: true }, &[0]),
                ]
            },
            &[(1, 3)],
            &[false],
        )),
    },
    Case {
        name: CaseName("error_of_the_last_consumer_retires_the_shared_batch"),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Stable,
            files: 1,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: false,
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0],
            const {
                &[
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Forwarded { complete: true }, &[1]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[0]),
                ]
            },
            &[(1, 3)],
            &[false],
        )),
    },
    Case {
        name: CaseName("cancellation_of_the_last_consumer_retires_the_shared_batch"),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Stable,
            files: 1,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: true,
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0],
            const {
                &[
                    seen(Answer::Failed(OperatingError::WrongOperation), &[1]),
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Refused(OperatingError::NotRead), &[0]),
                ]
            },
            &[(1, 3)],
            &[false],
        )),
    },
    Case {
        name: CaseName(
            "an_aliased_consumer_cancelled_early_does_not_retire_a_batch_other_rules_still_owe",
        ),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Stable,
            files: 1,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: true,
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Redeliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0],
            const {
                &[
                    seen(Answer::Rows(2), &[1]),
                    seen(Answer::Refused(OperatingError::NotRead), &[1]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[1]),
                    seen(Answer::Rows(2), &[0]),
                    seen(Answer::Repeated { same: true }, &[0]),
                ]
            },
            &[(1, 3)],
            &[false],
        )),
    },
    Case {
        name: CaseName("a_held_lease_stays_readable_across_reentrant_demands_and_retirement"),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Unstable,
            files: 1,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Hold,
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: false,
                    },
                    Step::Demand,
                    Step::Reread,
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0],
            const {
                &[
                    seen(Answer::Rows(2), &[3]),
                    seen(Answer::Held(2), &[3]),
                    seen(Answer::Rows(2), &[3]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[3]),
                    seen(Answer::Forwarded { complete: true }, &[0]),
                    seen(Answer::Demanded { found: false }, &[0]),
                    seen(Answer::Reread { same: true }, &[0]),
                ]
            },
            &[(3, 9)],
            &[false],
        )),
    },
    Case {
        name: CaseName("files_retire_their_own_batches_independently"),
        word: Word::Succeeds,
        input: || Script {
            residency: Residency::Stable,
            files: 2,
            steps: const {
                &[
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(1),
                    },
                    Step::Deliver {
                        file: 1,
                        rule: RuleIndex(1),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(3),
                    },
                    Step::Deliver {
                        file: 0,
                        rule: RuleIndex(0),
                    },
                    Step::Forward {
                        file: 0,
                        refuse: false,
                    },
                    Step::Deliver {
                        file: 1,
                        rule: RuleIndex(3),
                    },
                    Step::Deliver {
                        file: 1,
                        rule: RuleIndex(0),
                    },
                    Step::Forward {
                        file: 1,
                        refuse: false,
                    },
                ]
            },
        },
        expect: Expect::Value(trace(
            &[0, 0],
            const {
                &[
                    seen(Answer::Rows(2), &[1, 0]),
                    seen(Answer::Rows(2), &[1, 1]),
                    seen(Answer::Rows(2), &[1, 1]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[1, 1]),
                    seen(Answer::Forwarded { complete: true }, &[0, 1]),
                    seen(Answer::Rows(2), &[0, 1]),
                    seen(Answer::Failed(OperatingError::WrongOperation), &[0, 1]),
                    seen(Answer::Forwarded { complete: true }, &[0, 0]),
                ]
            },
            &[(1, 3), (1, 3)],
            &[false, false],
        )),
    },
];

/// Opens the script's files and runs its steps over them.
fn subject(input: Input) -> Result<Value, Failure> {
    script(input.residency, input.files, input.steps)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
