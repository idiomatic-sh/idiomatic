//! The types of the `operations` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeMap;

use crate::domain::violations::types::{Id, Key, MessageText, ToolId};

/// An operation's content address: the lowercase hex BLAKE3 digest of its LF text, its file's stem.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub Box<str>);

/// An operation's text, as its file holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OperationText(pub Box<str>);

/// How many violations share a key: above one when several of one rule sit on lines of one file whose text is the
/// same once whitespace is removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count(pub u32);

/// A day, `YYYY-MM-DD`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(pub Box<str>);

/// Why a violation was accepted.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reason(pub Box<str>);

/// A git commit, its full object id.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(pub Box<str>);

/// The commit a record was accepted at, when one was known.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Provenance {
    /// This commit.
    At(Revision),
    /// None was known.
    Unknown,
}

/// One accepted violation: its key, kept beside its id so a lock reads in review, and what is provenance or display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The content address of the key.
    pub id: Id,
    /// The key.
    pub key: Key,
    /// How many violations share the key.
    pub count: Count,
    /// The tool's message when it was accepted: display only.
    pub message: MessageText,
    /// The commit it was accepted at.
    pub since_rev: Provenance,
    /// The day it was accepted.
    pub since: Day,
    /// Why: more than one when branches accepted it concurrently; none for a suppressed record, whose reason is the
    /// in-source ignore beside its code.
    pub reasons: Vec<Reason>,
}

/// A record an operation removes: its key, and the id the key comes to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Removal {
    /// The content address of the key.
    pub id: Id,
    /// The key.
    pub key: Key,
}

/// One immutable accepted-state transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    /// The graph's heads when it was made; none for a snapshot, which replaces the operations it covers.
    pub parents: Vec<OperationId>,
    /// The operations a snapshot replaces, whose files it deletes: an operation whose parent is among them descends
    /// from what the snapshot holds, and is concurrent with it. None for any other operation.
    pub covers: Vec<OperationId>,
    /// The records it accepts.
    pub accepted: Vec<Record>,
    /// The records it removes, by key.
    pub removed: Vec<Removal>,
    /// The perf records it sets, each replacing the one of its key.
    pub perf: Vec<PerfRecord>,
    /// The perf records it removes, by key.
    pub perf_removed: Vec<PerfKey>,
}

/// The current state an operation graph folds to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folded {
    /// The accepted records, sorted by id.
    pub records: Vec<Record>,
    /// The removals that stand: ids whose maximal decisions include a removal, sorted by id.
    pub removals: Vec<Removal>,
    /// The operations no other names as a parent.
    pub heads: Vec<OperationId>,
    /// For each current record, the maximal operations that accepted it.
    pub origins: BTreeMap<Id, Vec<OperationId>>,
    /// The perf records, sorted by key.
    pub perf: Vec<PerfRecord>,
    /// The perf removals that stand: keys whose maximal decisions include a removal, sorted by key.
    pub perf_removals: Vec<PerfKey>,
}

/// Whether a folded lock is already one snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// One head that accepted every record it holds, or no operation at all.
    Compacted,
    /// Anything else.
    Open,
}

/// Whether one operation descends from another, itself included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Descent {
    /// It does.
    Descends,
    /// It does not.
    Apart,
}

/// An operation and one it may descend from: the key of what is already known of the two.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lineage {
    /// The later operation.
    pub operation: OperationId,
    /// The one it may descend from.
    pub ancestor: OperationId,
}

/// A maximal decision on a record: the operation that made it, and what it decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Maximal {
    /// The operation.
    pub operation: OperationId,
    /// What it decided.
    pub decision: Decision,
}

/// One decision on a record, made by an operation: accept it as this record, or remove it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Accept it.
    Accept(Record),
    /// Remove it.
    Remove,
}

/// A finite number a tool measured or a record holds. It is never NaN, so it is equal to itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Number(pub f64);

impl Eq for Number {}

/// A benchmark's name, as the tool's Bencher Metric Format names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BenchmarkName(pub Box<str>);

/// A measure's slug, as the tool's Bencher Metric Format names it: `wall-ms`, `latency`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasureName(pub Box<str>);

/// What a perf record is for: one measure of one benchmark of one tool.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerfKey {
    /// The tool.
    pub tool: ToolId,
    /// The benchmark.
    pub benchmark: BenchmarkName,
    /// The measure.
    pub measure: MeasureName,
}

/// The best a measure has reached, and how far from it a run may be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerfRecord {
    /// What it records.
    pub key: PerfKey,
    /// The best median.
    pub best: Number,
    /// Why this best stands, each reason `up` was given for it.
    pub reasons: Vec<Reason>,
}

/// One decision on a perf record, made by an operation: set it as this record, or remove it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PerfDecision {
    /// Set it.
    Set(PerfRecord),
    /// Remove it.
    Unset,
}

/// A maximal decision on a perf record: the operation that made it, and what it decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerfMaximal {
    /// The operation.
    pub operation: OperationId,
    /// What it decided.
    pub decision: PerfDecision,
}
