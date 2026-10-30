//! The types of the `perf` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operations::types::{
    BenchmarkName, Count, MeasureName, Number, PerfKey, PerfRecord,
};

/// What a perf outcome means for the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holding {
    /// Nothing fails and nothing is to be dropped.
    Holds,
    /// A median regressed or has no record, or a record is to be dropped.
    Breaks,
}

/// A bound of a measure's spread, when the tool gave one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spread {
    /// This one.
    Given(Number),
    /// None.
    Absent,
}

/// One measure of one benchmark: its median and, informationally, its spread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// The benchmark.
    pub benchmark: BenchmarkName,
    /// The measure.
    pub measure: MeasureName,
    /// The median, which is what is judged.
    pub value: Number,
    /// The spread's lower bound.
    pub lower: Spread,
    /// The spread's upper bound.
    pub upper: Spread,
}

/// A measurement and the tool that made it, under the key the lock records it by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measured {
    /// The tool, the benchmark and the measure.
    pub key: PerfKey,
    /// The median, which is what is judged.
    pub value: Number,
    /// The spread's lower bound.
    pub lower: Spread,
    /// The spread's upper bound.
    pub upper: Spread,
}

/// A run whose low end is above the best.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regression {
    /// The record it breaks.
    pub record: PerfRecord,
    /// The median.
    pub value: Number,
}

/// A median with no record to judge it by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unseeded {
    /// What it is a median of.
    pub key: PerfKey,
    /// The median.
    pub value: Number,
}

/// Whether a check rewrote the lock for the improvements it found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rewriting {
    /// It did: the bests are rewritten in the lock, to be committed.
    #[default]
    Written,
    /// It did not: a run that fixes, or that judges against a revision, never writes the lock.
    Withheld,
}

/// A run whose high end is below the best: the best is rewritten to its median.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Improvement {
    /// The record as it stood.
    pub record: PerfRecord,
    /// The median, the new best.
    pub value: Number,
}

/// What a run's measurements mean against the lock's perf records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerfOutcome {
    /// Runs whose low end is above their best.
    pub regressed: Vec<Regression>,
    /// Medians with no record.
    pub unseeded: Vec<Unseeded>,
    /// Runs whose high end is below their best.
    pub improved: Vec<Improvement>,
    /// Records whose tool ran and no longer reports them.
    pub resolved: Vec<PerfRecord>,
    /// Whether the lock was rewritten for the improvements.
    pub rewriting: Rewriting,
    /// How many runs were within their record.
    pub held: Count,
}
