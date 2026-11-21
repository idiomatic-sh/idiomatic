//! The types of the `masks` part: the free terms of a rule set's needs, and for each site the clauses of them its
//! formula cannot hold without.

use crate::domain::code::types::{PoolAt, RelationNumber, WordSlot};
use ryo_relations::domain::relations::types::Count;

/// A term of a need that costs nothing to ask of a file: a node of one of a list's kinds, or a row of a relation of
/// the node-table form (with a constant in a column when both are given).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FreeTerm {
    /// A node of one of the kinds of a list of the pool.
    Kinds(PoolAt),
    /// A row of a node relation.
    NodeRow {
        /// The relation.
        relation: RelationNumber,
        /// The column's place, or `NO_SLOT`.
        column: WordSlot,
        /// The constant's number, or `NO_SLOT`.
        constant: WordSlot,
    },
}

/// The number of a free term among a rule set's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TermNumber(pub(crate) u32);

/// Whether deciding a record may cost a fill, whatever the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RecordCost {
    /// Not found yet.
    #[default]
    Unknown,
    /// It costs nothing: kinds, node relations, and parts of them.
    Free,
    /// It may: a table, a filled relation, or a part that holds one.
    Dear,
}

/// What a rule set's needs say that costs nothing to ask, found once for a worker (the code does not change). Every
/// distinct free term has a number. For each site, clauses: each a list of terms of which SOME must hold in a file for
/// the site's formula to hold. They are what the formula needs, never all of it (a term that costs a fill says
/// nothing here): a site one of whose clauses has no term in the file cannot hold, and is decided without a walk of
/// its formula; a site whose clauses all have one is decided by the walk, as before.
#[derive(Debug, Default)]
pub(crate) struct NeedMasks {
    /// The free terms, by number.
    pub(crate) terms: Vec<FreeTerm>,
    /// The number of each free term met.
    pub(crate) numbers: hashbrown::HashMap<FreeTerm, TermNumber>,
    /// The clauses of each site, by the site's number, a clause its terms ascending: found at the site's first
    /// decision (none until then).
    pub(crate) of_site: Vec<Option<Vec<Vec<TermNumber>>>>,
    /// The sites being found: a formula that names itself says nothing of itself.
    pub(crate) open: hashbrown::HashSet<u32>,
    /// For each place among the needs, whether the record there may cost a fill; empty until the first decision.
    pub(crate) costs: Vec<RecordCost>,
}

/// A file's answers to the free terms, each asked at most once a file.
#[derive(Debug, Default)]
pub(crate) struct TermAnswers {
    /// The file the answers are of: the store's stamp.
    pub(crate) file: Count,
    /// For each term, two bits: whether it was asked, and whether it holds.
    pub(crate) bits: Vec<u8>,
}
