//! The types of the `ordering` pass: what it knows of one step of a level, what a sample says a use costs, and what a round
//! of the pass does at each step the lending walk lends.

use crate::domain::statistics::types::MeasuredUnit;
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_relations::domain::relations::types::Count;

/// How many times a step hands on for each time it runs: the share of its rows a test passes, the rows an iteration gives.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct PassRate(pub(crate) f64);

/// Counted work for each run: a step's own run and the runs of what it runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub(crate) struct RunWork(pub(crate) f64);

/// The rank a step is placed by: the lower, the earlier.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct StepRank(pub(crate) f64);

/// Whether a step must stay after another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepOrder {
    /// It must: one writes a register the other reads or writes.
    Held,
    /// It need not.
    Free,
}

/// What the pass knows of one step of a level it may reorder.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LevelStep {
    /// The registers it reads, its bodies' steps among them.
    pub(crate) reads: Vec<Register>,
    /// The registers it writes, its bodies' steps among them.
    pub(crate) writes: Vec<Register>,
    /// How many times it hands on for each run.
    pub(crate) rate: PassRate,
    /// What one run of it costs.
    pub(crate) work: RunWork,
}

/// Whether a step may be moved within its level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepPlacement {
    /// It may: it reads, tests or iterates.
    Free,
    /// It ends a row or hands the level on (`Yield`, `Emit`, `Fork`): it keeps its place, and so does every step after it.
    Held,
}

/// What of a step the order is decided by, beside its registers: whether it may move and the tables it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepFacts {
    /// Whether it may be moved.
    pub(crate) placement: StepPlacement,
    /// The made tables it reads rows of: a table's fill is work its first read in a file pays.
    pub(crate) tables: Vec<MadeTableId>,
}

/// What a use costs, as the sample counted it: of each definition and of each made table, by number.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct UseCosts {
    /// The counted work of one use of each definition.
    pub(crate) definitions: Vec<RunWork>,
    /// The counted work of one read of each made table.
    pub(crate) tables: Vec<RunWork>,
}

/// Where a program stands in its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LevelRole {
    /// A rule's branch: its first step is its anchor and keeps its place.
    Branch,
    /// A pattern's call form, an inline body or a slot: every step is keyed.
    Keyed,
}

/// What orders the levels of one unit: the costs of what its steps use, and the unit's own measured runs when the sample
/// measured it as it is lowered now and its order is not denied.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnitOrder<'s> {
    /// What a use costs.
    pub(crate) costs: &'s UseCosts,
    /// The unit as measured; none keeps the lowering's order.
    pub(crate) measured: Option<&'s MeasuredUnit>,
}

/// What a round does at a step the lending walk lends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoundEvent {
    /// A step above the round's depth: left as it is, and the walk goes into its bodies.
    Passed,
    /// A step at the round's depth: taken out of its place, and a step of its level put there.
    Moved,
}

/// One round of the pass: every level at one depth, in the lending walk's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OrderRound {
    /// What to do at each step the walk lends, in its order; a step below the round's depth is not among them.
    pub(crate) events: Vec<RoundEvent>,
    /// The order each level at the round's depth takes: for each place, the place its step holds now.
    pub(crate) levels: Vec<Vec<Count>>,
}
