//! The types of the `masks` part: the terms of a rule set's needs, what asking each costs, and a file's answers to
//! them.

use crate::domain::needs::formulas::types::{FreeValue, NeedFormulas};
use crate::domain::code::types::{PoolAt, RelationNumber, WordSlot};
use ryo_relations::domain::relations::types::Count;

/// A term of a need that is a fact of the file alone: a node of one of a list's kinds, or a row of a relation (with a
/// constant in a column when both are given). A row of a node relation costs nothing to ask; a row of a relation the
/// language fills costs its fill, once a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NeedTerm {
    /// A node of one of the kinds of a list of the pool.
    Kinds(PoolAt),
    /// A row of a relation.
    Row {
        /// The relation.
        relation: RelationNumber,
        /// The column's place, or `NO_SLOT`.
        column: WordSlot,
        /// The constant's number, or `NO_SLOT`.
        constant: WordSlot,
    },
}

/// The number of a term among a rule set's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TermNumber(pub(crate) u32);

/// A stretch of a list: where it starts and how many it holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stretch {
    /// Where it starts.
    pub(crate) start: Count,
    /// How many it holds.
    pub(crate) count: Count,
}

/// Whether deciding a record, or asking a term, may cost a fill, whatever the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RecordCost {
    /// Not found yet.
    #[default]
    Unknown,
    /// It costs nothing: kinds, node relations, and parts of them.
    Free,
    /// It may: a table, a filled relation, or a part that holds one.
    Dear,
    /// Of a term alone: a filled relation asked for ANY row, the least a relation can say of a file (it holds in
    /// nearly every file that has what the relation is about, and costs the relation's whole fill to learn). Asked
    /// last, after a site's tables: 43% of the whole fills of a run were read by nothing but such a term
    /// (build/x/WHERE-PROVIDER.md).
    Presence,
}

/// Whether a site's formula asks a relation of the node-table form for a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NodeAsk {
    /// Not found yet.
    #[default]
    Unknown,
    /// Being found: a formula that names itself adds nothing to itself.
    Finding,
    /// It asks one.
    Asked,
    /// It asks none.
    NotAsked,
}

/// Bits of one word of a file's free answers that must all be set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MustWord {
    /// Which word.
    pub(crate) word: Count,
    /// The bits.
    pub(crate) bits: u64,
}

/// What a rule set's needs say of a file in its terms, found once for a worker (the code does not change): every
/// distinct term has a number, the terms that cost nothing a place among the file's free bits, and each site's formula
/// its compiled nodes (`needs::formulas`).
#[derive(Debug, Default)]
pub(crate) struct NeedMasks {
    /// The terms, by number.
    pub(crate) terms: Vec<NeedTerm>,
    /// What asking each term may cost.
    pub(crate) term_costs: Vec<RecordCost>,
    /// The number of each term met.
    pub(crate) numbers: hashbrown::HashMap<NeedTerm, TermNumber>,
    /// For each term that costs nothing, its place among the free terms and one more; nought for a term that costs.
    pub(crate) free_places: Vec<u32>,
    /// The terms that cost nothing, in the order they were met: a file's free bits are in this order.
    pub(crate) free_terms: Vec<TermNumber>,
    /// For each place among the needs, whether the record there may cost a fill; empty until the first file.
    pub(crate) costs: Vec<RecordCost>,
    /// For each place among the needs whose record is a term, the term's number and one more; nought: none.
    pub(crate) leaves: Vec<u32>,
    /// For each site, whether its formula asks a relation of the node-table form for a row, whatever the file: found
    /// once a site.
    pub(crate) node_asks: Vec<NodeAsk>,
    /// Each site's formula compiled, at the site's first decision.
    pub(crate) formulas: NeedFormulas,
}

/// A file's answers to the terms, each asked at most once a file: a bit for "asked" and a bit for "holds".
#[derive(Debug, Default)]
pub(crate) struct TermAnswers {
    /// Whether each term was asked in this file, sixty-four terms a word.
    pub(crate) asked: Vec<u64>,
    /// Whether each term asked holds.
    pub(crate) holds: Vec<u64>,
    /// The file's answers to the free terms, a bit a term in the order of the masks' free terms: every free term met
    /// so far is asked once, together, at the file's first decision.
    pub(crate) free: Vec<u64>,
    /// How many of the free terms `free` holds the answers of.
    pub(crate) free_known: Count,
    /// What each site's formula says of the file from the free terms alone, by the site's number: found at most once a
    /// file.
    pub(crate) sites: Vec<FreeValue>,
}
