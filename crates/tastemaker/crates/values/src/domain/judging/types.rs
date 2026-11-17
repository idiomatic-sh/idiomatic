//! The types of the `judging` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! The rows tm hosts for the count and closed queries, and the rows they answer.

use crate::domain::entries::types::{Amount, ConditionName};
use crate::domain::scopes::types::{Answer, Identity, ScopeIndex};

/// The number a count or a pair has among the ones one count call judges, from zero: the counts first, then the pairs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryNumber(pub u32);

/// What a row of `tastemaker.template-entry` is judged for: a count held to the number found, or a pair whose findings
/// stand while its count requires any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A count: `found` is held to its bounds.
    Count,
    /// A pair: its findings stand when the count's least is more than none.
    Pair,
}

/// The bounds of a count, as the rows write them: the most `u32::MAX` when it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The fewest.
    pub min: Amount,
    /// The most.
    pub max: Amount,
}

/// A row of `tastemaker.template-entry`: one count or pair a placement left waiting, its bounds when it reads no
/// condition, and those of each branch when it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryRow {
    /// Its number in the call.
    pub entry: EntryNumber,
    /// The scope whose conditions it reads.
    pub scope: ScopeIndex,
    /// The entry that counts.
    pub identity: Identity,
    /// What it is judged for.
    pub kind: EntryKind,
    /// How many were found; none for a pair.
    pub found: Amount,
    /// The condition it reads; blank when it reads none.
    pub condition: ConditionName,
    /// Its bounds when it reads no condition.
    pub bounds: Bounds,
    /// Its bounds when the condition holds.
    pub then: Bounds,
    /// Its bounds when it does not.
    pub otherwise: Bounds,
}

/// A row of `tastemaker.condition-answer`: what a condition a count reads answered in its scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionRow {
    /// The scope whose composition declares it.
    pub scope: ScopeIndex,
    /// The condition.
    pub condition: ConditionName,
    /// What its rule answered.
    pub holds: Answer,
}

/// What a check's counts and pairs host for the count queries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CountRows {
    /// `tastemaker.template-entry`.
    pub entries: Vec<EntryRow>,
    /// `tastemaker.condition-answer`.
    pub conditions: Vec<ConditionRow>,
    /// The closed directories and what they hold, for the closed query.
    pub closings: crate::domain::closing::types::Closings,
}

/// Which bounds a count was held to: its own, or those of the branch its condition's answer picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoundsHeld {
    /// Its own: it reads no condition.
    Own,
    /// The `then` branch: the condition holds.
    Then,
    /// The `else` branch: it does not.
    Otherwise,
}

/// A row the count queries answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Judgment {
    /// The count's found number is outside the bounds it was held to.
    Refused {
        /// The count.
        entry: EntryNumber,
        /// The bounds it was held to.
        held: BoundsHeld,
    },
    /// The pair's count requires its files: its findings stand.
    Missing {
        /// The pair.
        entry: EntryNumber,
    },
    /// A file of a closed directory that nothing places, and the scope it is named after.
    Stray(crate::domain::closing::types::Stray),
}
