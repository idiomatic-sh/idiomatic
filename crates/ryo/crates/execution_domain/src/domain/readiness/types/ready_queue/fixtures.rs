//! The driver of the readiness queue's cases: it runs a scripted session or a chain against the
//! queue and returns what the queue dispatched, refused and counted.
use super::value::ReadyQueue;
use crate::domain::{
    executable_program::types::{RegionCount, RegionId},
    operating::errors::OperatingError,
    readiness::types::{
        ReadinessAction as Action, ReadinessCounts, ReadinessEntryCount, ReadinessEvents,
        ReadinessOutcome as Outcome,
    },
};
use std::borrow::Cow;

/// One step a driver takes against the queue.
#[derive(Clone, Copy, Debug)]
pub(super) enum Step {
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
pub(super) enum Event {
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
pub(super) struct Session {
    pub(super) regions: usize,
    pub(super) roots: &'static [RegionId],
    pub(super) steps: &'static [Step],
}

/// What a driver asks of the queue.
#[derive(Debug)]
pub(super) enum Drive {
    /// A scripted session.
    Session(Session),
    /// A chain of this many regions from region 0, each waiting on the next, answered until it ends.
    Chain(usize),
}

/// What a session observed: each step's events, the action of the last poll, and the queue's counts.
#[derive(Debug, PartialEq)]
pub(super) struct Trace {
    pub(super) events: Cow<'static, [Event]>,
    pub(super) end: Option<Action>,
    pub(super) counts: ReadinessCounts,
}

/// What a chain observed until the queue had no action left: how many actions it dispatched, whether
/// they were the chain's order (every region prepared from the root down, then each evaluated from the
/// deepest up, prepared again first unless it is the deepest), and the counts.
#[derive(Debug, PartialEq)]
pub(super) struct Chain {
    pub(super) dispatched: usize,
    pub(super) in_chain_order: bool,
    pub(super) counts: ReadinessCounts,
}

/// What a drive answers.
#[derive(Debug, PartialEq)]
pub(super) enum Answer {
    Trace(Trace),
    Chain(Chain),
}

/// A session the queue refused on its last poll: the events before it and the refusal.
#[derive(Debug, PartialEq)]
pub(super) struct Halted {
    pub(super) events: Cow<'static, [Event]>,
    pub(super) error: OperatingError,
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

pub(super) fn drive_session(session: &Session) -> Result<Answer, Halted> {
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

pub(super) fn drive_chain(depth: usize) -> Result<Answer, Halted> {
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
pub(super) const fn counts(
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

pub(super) const fn prepare(id: usize) -> Event {
    Event::Dispatched(Action::Prepare(RegionId(id)))
}

pub(super) const fn evaluate(id: usize) -> Event {
    Event::Dispatched(Action::Evaluate(RegionId(id)))
}

pub(super) const fn session(
    regions: usize,
    roots: &'static [RegionId],
    steps: &'static [Step],
) -> Drive {
    Drive::Session(Session {
        regions,
        roots,
        steps,
    })
}
