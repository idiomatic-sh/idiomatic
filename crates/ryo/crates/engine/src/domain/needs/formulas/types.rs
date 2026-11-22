//! The types of the `formulas` part: a site's formula as nodes, each part ranked by what asking it may cost.

use crate::domain::code::types::NeedSite;
use crate::domain::needs::masks::types::{MustWord, Stretch, TermNumber};
use crate::domain::steps::types::MadeTableId;

/// What asking a node may cost at most, in the order the parts of a node are asked: a part that costs less is asked
/// before one that costs more, so that a dearer part is asked only when the cheaper ones left the node undecided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AskRank {
    /// Nothing: a bit of the file's free answers.
    #[default]
    Free,
    /// A relation the language fills, asked for a value in a column.
    Row,
    /// A table made, or another site's decision that may make one.
    Table,
    /// A relation the language fills, asked for any row at all: its whole fill, for the least a relation can say of a
    /// file. Asked last, when nothing else decided.
    Presence,
}

/// One node of a compiled formula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormulaNode {
    /// It holds: nothing is known before the unit runs, or a record that cannot be read.
    Always,
    /// It never holds: an `Any` of no part.
    Never,
    /// A term that costs nothing: its place among the free terms, the bit of the file's free answers.
    Free(u32),
    /// A term that costs a fill: its number.
    Term(TermNumber),
    /// A table's row, made when its own need holds.
    Table(MadeTableId),
    /// Another unit's decision.
    Site(NeedSite),
    /// Every part holds: a stretch of the formulas' parts, cheapest first.
    All(Stretch),
    /// Some part holds: a stretch of the formulas' parts, cheapest first.
    Any(Stretch),
    /// Every one of several free terms holds: a stretch of the formulas' bit words, each of whose bits must be set.
    AllBits(Stretch),
    /// Some one of several free terms holds: a stretch of the formulas' bit words, one of whose bits must be set.
    AnyBits(Stretch),
}

/// The formulas of a rule set's needs, compiled once a worker, each site at its first decision.
#[derive(Debug, Default)]
pub(crate) struct NeedFormulas {
    /// The nodes, by number.
    pub(crate) nodes: Vec<FormulaNode>,
    /// What asking each node may cost.
    pub(crate) ranks: Vec<AskRank>,
    /// The parts of every `All` and `Any`, one node after the other, each a node's number.
    pub(crate) parts: Vec<u32>,
    /// The bit words of every `AllBits` and `AnyBits`, one node after the other.
    pub(crate) bits: Vec<MustWord>,
    /// For each place among the needs, the number of the node compiled from the record there and one more; nought:
    /// none yet.
    pub(crate) at_place: Vec<u32>,
    /// For each site, the number of the node of its formula and one more; nought: not compiled; `u32::MAX`: being
    /// compiled (a formula that names itself).
    pub(crate) roots: Vec<u32>,
}

/// What a site's formula says of a file from the terms that cost nothing (and from what the file already answered),
/// kept for the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FreeValue {
    /// Not found yet in this file.
    #[default]
    Unfound,
    /// It cannot hold, whatever the terms that cost say.
    Empty,
    /// It holds, whatever the terms that cost say.
    Holds,
    /// The terms that cost decide.
    Open,
    /// Being decided: a formula met inside itself says nothing of itself.
    Deciding,
}
