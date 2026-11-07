//! The types of the `judgement` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeSet;

use crate::domain::manifest::types::Seconds;
use crate::domain::operations::types::{Count, Record};
use crate::domain::violations::types::{Key, RuleId, ToolId, Violation};

/// What a reader of the tool, its parser or the system, said went wrong.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Problem(pub Box<str>);

/// The last lines a tool wrote to stderr.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tail(pub Box<str>);

/// A process's exit code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(pub i32);

/// How a tool's run failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// It exited with a code that is no verdict: anything but 0 for an exit-status tool, 2 or more for a SARIF one.
    Exited {
        /// The code.
        code: Code,
        /// What it last said.
        stderr: Tail,
    },
    /// A signal ended it.
    Signalled {
        /// What it last said.
        stderr: Tail,
    },
    /// It ran past its timeout, and everything it started was killed.
    TimedOut {
        /// The timeout.
        after: Seconds,
    },
    /// It could not start.
    Unstartable(Problem),
    /// Its output is no SARIF log: no output at all included.
    Unreadable {
        /// What the parser said.
        problem: Problem,
        /// What it last said.
        stderr: Tail,
    },
}

/// Whether a tool's output can be believed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// It ran and its output was understood.
    Completed,
    /// It failed: what it would have said is unknown, never clean.
    Failed(Failure),
}

/// Which rules a run says it evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inventory {
    /// Every rule it evaluated: SARIF's `tool.driver.rules`, from a run that says it completed.
    Declared(BTreeSet<RuleId>),
    /// It cannot say.
    Undeclared,
}

/// What one tool's run shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// The tool.
    pub tool: ToolId,
    /// Whether its output can be believed.
    pub status: Status,
    /// The rules it says it evaluated.
    pub inventory: Inventory,
    /// The rules it reported a violation of.
    pub reported: BTreeSet<RuleId>,
}

/// What shows an accepted violation no longer occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Its rule was evaluated and reported fewer than were accepted.
    NotReported,
    /// Its tool ran, declared every rule it evaluates, and its rule is not among them: it can never be reported
    /// again, so it is resolved rather than left unjudged.
    RuleRetired,
}

/// A record some or all of whose violations no longer occur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The record.
    pub record: Record,
    /// How many of its violations no longer occur.
    pub count: Count,
    /// How many still do.
    pub remaining: Count,
    /// What shows it.
    pub resolution: Resolution,
}

/// Why a record cannot be judged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaleReason {
    /// Its tool is configured and did not run.
    ToolNotRun,
    /// Its tool ran and failed.
    ToolFailed,
    /// Its tool ran, but nothing shows its rule was evaluated.
    RuleNotEvaluated,
}

/// A record that cannot be judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stale {
    /// The record.
    pub record: Record,
    /// Why.
    pub reason: StaleReason,
}

/// What a run means against the accepted records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Judgement {
    /// The violations nothing accepts.
    pub new: Vec<Violation>,
    /// The violations an accepted record covers.
    pub accepted: Vec<Violation>,
    /// The records some of whose violations no longer occur.
    pub resolved: Vec<Resolved>,
    /// The records that cannot be judged.
    pub stale: Vec<Stale>,
    /// The records some of whose violations the run reports at another path: their code moved there.
    pub relocated: Vec<Relocated>,
}

/// Violations of a record the run reports under another key than the record's, at another path: code that moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relocated {
    /// The record.
    pub record: Record,
    /// The key the violations are reported under now.
    pub to: Key,
    /// How many of them.
    pub count: Count,
}

/// What a run comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing new, nothing resolved, everything judged.
    Pass,
    /// Something new, or a record the lock must drop.
    Changed,
    /// A tool failed or did not run: what it would have said is unknown.
    Undetermined,
}

/// Whether a record's rule can be shown to have been evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// A completed run of its tool evaluated it.
    Evaluated,
    /// Every run of its tool declared its rules, and none holds or reported it.
    Retired,
    /// Nothing shows it either way.
    Missing(StaleReason),
}

/// What identifies a violation wherever it is: its key without its path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Content<'a> {
    /// The tool.
    pub tool: &'a ToolId,
    /// The rule.
    pub rule: &'a RuleId,
    /// How serious.
    pub level: crate::domain::violations::types::Level,
    /// Whether an ignore suppresses it.
    pub standing: crate::domain::violations::types::Standing,
    /// Its fingerprint.
    pub fingerprint: crate::domain::violations::types::Fingerprint,
}

/// A record its own path reports fewer times than it accepts, before any violation that moved is counted.
#[derive(Clone, Copy, Debug)]
pub struct Short<'a> {
    /// The record.
    pub record: &'a Record,
    /// How many it accepts.
    pub accepted: Count,
    /// How many are seen.
    pub seen: Count,
}

/// The sightings of one key no record at its path accepts, each taken when a moved record claims it.
#[derive(Clone, Debug)]
pub struct Sightings<'a> {
    /// The key.
    pub key: &'a Key,
    /// Its sightings, each `None` once claimed.
    pub violations: Vec<Option<&'a Violation>>,
}

/// A place in a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(pub usize);

/// Where one sighting is among the pending ones.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    /// Its key's place.
    pub group: Position,
    /// Its place among the key's sightings.
    pub index: Position,
}
