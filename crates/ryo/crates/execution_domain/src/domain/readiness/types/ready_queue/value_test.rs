//! Transition controls for the production readiness queue, without a second evaluator.
use super::fixtures::{
    Answer, Chain, Drive, Event, Halted, Step, Trace, counts, drive_chain, drive_session, evaluate,
    prepare, session,
};
use crate::domain::{
    executable_program::types::RegionId,
    operating::errors::OperatingError,
    readiness::types::ReadinessEntryCount,
    readiness::types::ReadinessOutcome as Outcome,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

type Input = Drive;
type Value = Answer;
type Failure = Halted;

fn subject(drive: Input) -> Result<Value, Failure> {
    match drive {
        Drive::Session(session) => drive_session(&session),
        Drive::Chain(depth) => drive_chain(depth),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("large unreached program allocates only reached readiness entries"),
        word: Word::Succeeds,
        input: || {
            session(
                10_000_000,
                &[RegionId(7), RegionId(7)],
                &[
                    Step::Entries,
                    Step::Answer(Outcome::Waiting(RegionId(10_000_000))),
                    Step::Entries,
                    Step::Observe(RegionId(7), Outcome::Waiting(RegionId(9_999_999))),
                    Step::Entries,
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Terminal),
                    Step::Entries,
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        Event::Entries(ReadinessEntryCount(1)),
                        prepare(7),
                        Event::Refused(OperatingError::NotRun),
                        Event::Entries(ReadinessEntryCount(1)),
                        Event::Entries(ReadinessEntryCount(2)),
                        prepare(9_999_999),
                        prepare(7),
                        Event::Entries(ReadinessEntryCount(2)),
                    ]
                },
            ),
            end: None,
            counts: counts(3, 0, 1, 1),
        })),
    },
    Case {
        name: CaseName("preparation terminal wakes before ready evaluation"),
        word: Word::Succeeds,
        input: || {
            session(
                3,
                &[RegionId(0), RegionId(1)],
                &[
                    Step::Answer(Outcome::Waiting(RegionId(2))),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        prepare(0),
                        prepare(1),
                        prepare(2),
                        prepare(0),
                        evaluate(1),
                        evaluate(0),
                    ]
                },
            ),
            end: None,
            counts: counts(4, 2, 1, 1),
        })),
    },
    Case {
        name: CaseName("waiting on a terminal producer refuses instead of spinning"),
        word: Word::Succeeds,
        input: || {
            session(
                2,
                &[RegionId(0), RegionId(1)],
                &[
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Waiting(RegionId(0))),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        prepare(0),
                        prepare(1),
                        Event::Refused(OperatingError::NotRun),
                    ]
                },
            ),
            end: None,
            counts: counts(2, 0, 1, 0),
        })),
    },
    Case {
        name: CaseName("diamond and duplicate roots preserve cohort order"),
        word: Word::Succeeds,
        input: || {
            session(
                4,
                &[RegionId(0), RegionId(1), RegionId(0)],
                &[
                    Step::Answer(Outcome::Waiting(RegionId(2))),
                    Step::Answer(Outcome::Waiting(RegionId(2))),
                    Step::Answer(Outcome::Waiting(RegionId(3))),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        prepare(0),
                        prepare(1),
                        prepare(2),
                        prepare(3),
                        evaluate(3),
                        prepare(2),
                        evaluate(2),
                        prepare(0),
                        prepare(1),
                        evaluate(0),
                        evaluate(1),
                    ]
                },
            ),
            end: None,
            counts: counts(7, 4, 3, 3),
        })),
    },
    Case {
        name: CaseName("deep chain has linear admission work"),
        word: Word::Succeeds,
        input: || Drive::Chain(2048),
        expect: Expect::Value(Answer::Chain(Chain {
            dispatched: 6143,
            in_chain_order: true,
            counts: counts(4095, 2048, 2047, 2047),
        })),
    },
    Case {
        name: CaseName("terminal failure wakes exact waiters"),
        word: Word::Succeeds,
        input: || {
            session(
                3,
                &[RegionId(0), RegionId(1)],
                &[
                    Step::Answer(Outcome::Waiting(RegionId(2))),
                    Step::Answer(Outcome::Waiting(RegionId(2))),
                    Step::Answer(Outcome::Ready),
                    // A failed actual poll and a published actual poll both produce Terminal.
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        prepare(0),
                        prepare(1),
                        prepare(2),
                        evaluate(2),
                        prepare(0),
                        prepare(1),
                    ]
                },
            ),
            end: None,
            counts: counts(5, 1, 2, 2),
        })),
    },
    Case {
        name: CaseName("unreached nodes stay unpolled and cycles refuse"),
        word: Word::Succeeds,
        input: || {
            session(
                3,
                &[RegionId(0)],
                &[
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(const { &[prepare(0), evaluate(0)] }),
            end: None,
            counts: counts(1, 1, 0, 0),
        })),
    },
    Case {
        name: CaseName("a cycle refuses"),
        word: Word::Refuses,
        input: || {
            session(
                2,
                &[RegionId(0)],
                &[
                    Step::Answer(Outcome::Waiting(RegionId(1))),
                    Step::Answer(Outcome::Waiting(RegionId(0))),
                ],
            )
        },
        expect: Expect::Error(Halted {
            events: Cow::Borrowed(const { &[prepare(0), prepare(1)] }),
            error: OperatingError::NotRun,
        }),
    },
    Case {
        name: CaseName("an evaluation in preparation is counted once by the queue"),
        word: Word::Succeeds,
        input: || {
            // A region that joins no shared walk is evaluated inside its preparation poll and suspends on nested
            // demand; its resumed evaluation is dispatched.
            session(
                2,
                &[RegionId(0)],
                &[
                    Step::AnswerInline(Outcome::Waiting(RegionId(1))),
                    Step::AnswerInline(Outcome::Terminal),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(const { &[prepare(0), prepare(1), prepare(0), evaluate(0)] }),
            end: None,
            counts: counts(3, 3, 1, 1),
        })),
    },
    Case {
        name: CaseName("the same region evaluated only by dispatch counts the same"),
        word: Word::Succeeds,
        input: || {
            session(
                2,
                &[RegionId(0)],
                &[
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Waiting(RegionId(1))),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                    Step::Answer(Outcome::Ready),
                    Step::Answer(Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        prepare(0),
                        evaluate(0),
                        prepare(1),
                        evaluate(1),
                        prepare(0),
                        evaluate(0),
                    ]
                },
            ),
            end: None,
            counts: counts(3, 3, 1, 1),
        })),
    },
    Case {
        name: CaseName("an evaluation in preparation is recorded only for a preparation poll"),
        word: Word::Succeeds,
        input: || {
            session(
                1,
                &[RegionId(0)],
                &[
                    // Not polling: nothing to record.
                    Step::Record(RegionId(0)),
                    Step::Poll,
                    Step::Record(RegionId(0)),
                    // The poll is an evaluation now: a second record would count it twice.
                    Step::Record(RegionId(0)),
                    Step::Observe(RegionId(0), Outcome::Ready),
                    Step::Poll,
                    // A dispatched evaluation is already counted.
                    Step::Record(RegionId(0)),
                    Step::Observe(RegionId(0), Outcome::Terminal),
                ],
            )
        },
        expect: Expect::Value(Answer::Trace(Trace {
            events: Cow::Borrowed(
                const {
                    &[
                        Event::Refused(OperatingError::NotRun),
                        prepare(0),
                        Event::Recorded,
                        Event::Refused(OperatingError::NotRun),
                        evaluate(0),
                        Event::Refused(OperatingError::NotRun),
                    ]
                },
            ),
            end: None,
            counts: counts(1, 2, 0, 0),
        })),
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
