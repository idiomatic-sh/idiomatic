// ignore: idiomatic.arch/file-length - one case table for the one readiness queue: every row is a script of the same driver and states every action the queue dispatched, so a row is as long as its script.
//! Transition controls for the production readiness queue, without a second evaluator.
use super::value::ReadyQueue;
use crate::domain::{
    executable_program::types::{RegionCount, RegionId},
    operating::errors::OperatingError,
    readiness::types::{
        ReadinessAction as Action, ReadinessCounts, ReadinessEntryCount, ReadinessEvents,
        ReadinessOutcome as Outcome,
    },
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// One step a driver takes against the queue.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Polls the action the queue dispatches and answers it with the outcome.
    Answer(Outcome),
    /// Polls the action, records that its poll evaluated the region in preparation, and answers it.
    AnswerInline(Outcome),
    /// Polls the action the queue dispatches and leaves it unanswered.
    Poll,
    /// Answers a region's poll with the outcome.
    Observe(RegionId, Outcome),
    /// Records that the region's poll evaluated it in preparation.
    Record(RegionId),
    /// Reads how many readiness entries the queue has allocated.
    Entries,
}

/// What a step observed.
#[derive(Clone, Debug, PartialEq)]
enum Event {
    /// The queue dispatched this action.
    Dispatched(Action),
    /// The queue had no action left.
    Idle,
    /// The queue recorded an evaluation in preparation.
    Recorded,
    /// The queue refused the step.
    Refused(OperatingError),
    /// The queue's readiness entries.
    Entries(ReadinessEntryCount),
}

/// A queue over `regions` regions with these roots, driven by these steps and then polled once more.
#[derive(Debug)]
struct Session {
    regions: usize,
    roots: &'static [RegionId],
    steps: &'static [Step],
}

/// What a driver asks of the queue.
#[derive(Debug)]
enum Drive {
    /// A scripted session.
    Session(Session),
    /// A chain of this many regions from region 0, each waiting on the next, answered until it ends.
    Chain(usize),
}

/// What a session observed: each step's events, the action of the last poll, and the queue's counts.
#[derive(Debug, PartialEq)]
struct Trace {
    events: Cow<'static, [Event]>,
    end: Option<Action>,
    counts: ReadinessCounts,
}

/// What a chain observed until the queue had no action left: how many actions it dispatched, whether
/// they were the chain's order (every region prepared from the root down, then each evaluated from the
/// deepest up, prepared again first unless it is the deepest), and the counts.
#[derive(Debug, PartialEq)]
struct Chain {
    dispatched: usize,
    in_chain_order: bool,
    counts: ReadinessCounts,
}

/// What a drive answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Trace(Trace),
    Chain(Chain),
}

/// A session the queue refused on its last poll: the events before it and the refusal.
#[derive(Debug, PartialEq)]
struct Halted {
    events: Cow<'static, [Event]>,
    error: OperatingError,
}

type Input = Drive;
type Value = Answer;
type Failure = Halted;

fn subject(drive: Input) -> Result<Value, Failure> {
    match drive {
        Drive::Session(session) => drive_session(&session),
        Drive::Chain(depth) => drive_chain(depth),
    }
}

/// The region an action is for.
fn region(action: Action) -> RegionId {
    match action {
        Action::Prepare(id) | Action::Evaluate(id) => id,
    }
}

/// Keeps a refusal among the events.
fn refused(events: &mut Vec<Event>, result: Result<(), OperatingError>) {
    if let Err(error) = result {
        events.push(Event::Refused(error));
    }
}

fn drive_session(session: &Session) -> Result<Answer, Halted> {
    let mut events = Vec::new();
    let mut queue =
        ReadyQueue::new(RegionCount(session.regions), session.roots).map_err(|error| Halted {
            events: Cow::Borrowed(&[]),
            error,
        })?;
    for &step in session.steps {
        match step {
            Step::Answer(outcome) | Step::AnswerInline(outcome) => match queue.next() {
                Ok(Some(action)) => {
                    events.push(Event::Dispatched(action));
                    if matches!(step, Step::AnswerInline(_)) {
                        let recorded = queue.evaluated_in_preparation(region(action));
                        refused(&mut events, recorded);
                    }
                    let observed = queue.observe(region(action), outcome);
                    refused(&mut events, observed);
                }
                Ok(None) => events.push(Event::Idle),
                Err(error) => events.push(Event::Refused(error)),
            },
            Step::Poll => match queue.next() {
                Ok(Some(action)) => events.push(Event::Dispatched(action)),
                Ok(None) => events.push(Event::Idle),
                Err(error) => events.push(Event::Refused(error)),
            },
            Step::Observe(id, outcome) => {
                let observed = queue.observe(id, outcome);
                refused(&mut events, observed);
            }
            Step::Record(id) => match queue.evaluated_in_preparation(id) {
                Ok(()) => events.push(Event::Recorded),
                Err(error) => events.push(Event::Refused(error)),
            },
            Step::Entries => events.push(Event::Entries(queue.reached_entries())),
        }
    }
    let end = queue.next();
    let events: Cow<'static, [Event]> = Cow::Owned(events);
    end.map(|end| {
        Answer::Trace(Trace {
            events: events.clone(),
            end,
            counts: queue.counts(),
        })
    })
    .map_err(|error| Halted { events, error })
}

fn drive_chain(depth: usize) -> Result<Answer, Halted> {
    let halted = |error| Halted {
        events: Cow::Borrowed(&[]),
        error,
    };
    let mut queue = ReadyQueue::new(RegionCount(depth), &[RegionId(0)]).map_err(halted)?;
    let mut dispatched = Vec::new();
    while let Some(action) = queue.next().map_err(halted)? {
        dispatched.push(action);
        let outcome = match action {
            Action::Prepare(RegionId(id)) if id + 1 < depth && id + 1 == dispatched.len() => {
                Outcome::Waiting(RegionId(id + 1))
            }
            Action::Prepare(_) => Outcome::Ready,
            Action::Evaluate(_) => Outcome::Terminal,
        };
        queue.observe(region(action), outcome).map_err(halted)?;
    }
    let order =
        (0..depth)
            .map(|id| Action::Prepare(RegionId(id)))
            .chain((0..depth).rev().flat_map(|id| {
                let again = (id + 1 != depth).then_some(Action::Prepare(RegionId(id)));
                again.into_iter().chain([Action::Evaluate(RegionId(id))])
            }));
    Ok(Answer::Chain(Chain {
        dispatched: dispatched.len(),
        in_chain_order: dispatched.iter().copied().eq(order),
        counts: queue.counts(),
    }))
}

/// The counts of a queue: preparations, evaluations, suspensions and wakeups; none published empty.
const fn counts(
    preparations: u64,
    evaluations: u64,
    suspensions: u64,
    wakeups: u64,
) -> ReadinessCounts {
    ReadinessCounts {
        preparations: ReadinessEvents(preparations),
        evaluations: ReadinessEvents(evaluations),
        suspensions: ReadinessEvents(suspensions),
        wakeups: ReadinessEvents(wakeups),
        presence_absent: ReadinessEvents(0),
    }
}

const fn prepare(id: usize) -> Event {
    Event::Dispatched(Action::Prepare(RegionId(id)))
}

const fn evaluate(id: usize) -> Event {
    Event::Dispatched(Action::Evaluate(RegionId(id)))
}

const fn session(regions: usize, roots: &'static [RegionId], steps: &'static [Step]) -> Drive {
    Drive::Session(Session {
        regions,
        roots,
        steps,
    })
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
