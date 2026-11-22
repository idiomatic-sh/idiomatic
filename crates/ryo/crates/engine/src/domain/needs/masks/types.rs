//! The types of the `masks` part: the terms of a rule set's needs, and for each site the clauses of them its formula
//! cannot hold without.

use crate::domain::needs::formulas::types::{FreeValue, NeedFormulas};
use crate::domain::code::types::{PoolAt, RelationNumber, WordSlot};
use crate::domain::indexes::types::FileStamp;
use ryo_language::domain::facts::types::CellWord;
use crate::domain::steps::types::MadeTableId;
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

/// Whether a site's clauses are all its formula says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MaskExact {
    /// They are what it needs and more is asked that is not kept here: a table under an `Any`, or what was left out
    /// past a bound. When they all hold the formula is walked.
    #[default]
    Part,
    /// They are the formula: when they all hold and every table of the site has a row, it holds.
    Whole,
}

/// Where what the masks hold of one site lies: its clauses (every one must have a term in the file), its groups (in
/// every one some alternative must have all its clauses hold: an `Any` of parts kept as it is, never multiplied out),
/// and the tables its formula cannot hold without a row of, in the formula's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SiteStretches {
    /// Its clauses: a stretch of the masks' clauses.
    pub(crate) clauses: Stretch,
    /// Its groups: a stretch of the masks' groups.
    pub(crate) groups: Stretch,
    /// Its tables: a stretch of the masks' tables.
    pub(crate) tables: Stretch,
    /// The free terms every one of which it needs (its clauses of one free term), as words of the file's free bits:
    /// a stretch of the masks' `must`. One and-compare a word decides most sites.
    pub(crate) must: Stretch,
}

/// Bits of one word of a file's free answers that must all be set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MustWord {
    /// Which word.
    pub(crate) word: Count,
    /// The bits.
    pub(crate) bits: u64,
}

/// What the masks hold of one site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SiteMasks {
    /// Not found yet: found at the site's first decision.
    #[default]
    Unfound,
    /// Its clauses, groups and tables, and whether they are all of its formula.
    Found(SiteStretches, MaskExact),
}

/// What a rule set's needs say of a file in its terms, found once for a worker (the code does not change). Every
/// distinct term has a number. For each site, clauses: each a list of terms of which SOME must hold in a file for the
/// site's formula to hold; the clauses of terms that cost nothing first, and in a clause those terms first. A site
/// one of whose clauses has no term in the file cannot hold, and is decided without a walk of its formula and without
/// making a table; a site whose clauses are its whole formula and all hold is decided too; any other is walked.
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
    /// The words of every site's `must`, one site after the other.
    pub(crate) must: Vec<MustWord>,
    /// What is held of each site, by the site's number.
    pub(crate) sites: Vec<SiteMasks>,
    /// Each clause's stretch of `clause_terms`.
    pub(crate) clauses: Vec<Stretch>,
    /// The terms of every clause, one clause after the other.
    pub(crate) clause_terms: Vec<TermNumber>,
    /// Each alternative's clauses: a stretch of `clauses`.
    pub(crate) alternatives: Vec<Stretch>,
    /// Each group's alternatives: a stretch of `alternatives`.
    pub(crate) groups: Vec<Stretch>,
    /// The tables of every site, one site after the other.
    pub(crate) tables: Vec<MadeTableId>,
    /// The sites being found: a formula that names itself says nothing of itself.
    pub(crate) open: hashbrown::HashSet<u32>,
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
    /// The values of the filled columns a term asked for a constant, each column's found once a file.
    pub(crate) values: Vec<ColumnValues>,
}

/// The distinct values of one column of a filled relation in one file, ascending: what a term that asks the column for a
/// constant searches.
#[derive(Debug)]
pub(crate) struct ColumnValues {
    /// The relation.
    pub(crate) relation: RelationNumber,
    /// The column's place.
    pub(crate) column: WordSlot,
    /// The file the values are of.
    pub(crate) stamp: FileStamp,
    /// The values.
    pub(crate) cells: Vec<CellWord>,
}
