//! The types of the `shapes` part: a shape read back from the pool, the vocabulary its words are read in, and what
//! deciding it counted.

use crate::domain::code::types::WordSlot;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// One element of a star as the pool writes it: its kind's and its field's constants (`NO_SLOT`: none), and whether it
/// is the own tree's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WrittenNode {
    /// The kind's constant, or `NO_SLOT`.
    pub(crate) kind: WordSlot,
    /// The field's constant, or `NO_SLOT`.
    pub(crate) field: WordSlot,
    /// Whether it is an element of the file's own tree.
    pub(crate) own: OwnTree,
}

/// Whether an element must be of the file's own tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnTree {
    /// Any element.
    Any,
    /// An element of the own tree alone.
    Own,
}

/// A star as the pool writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WrittenStar {
    /// The element entered at.
    pub(crate) anchor: WrittenNode,
    /// Its parent, when held.
    pub(crate) parent: Option<WrittenNode>,
    /// Its children held.
    pub(crate) children: Vec<WrittenNode>,
    /// The kinds' constants of the ancestors held.
    pub(crate) ancestors: Vec<WordSlot>,
}

/// The words a shape's constants are read in: the run's constants, and the node table's kind and field names by code.
pub(crate) struct ShapeWords<'a> {
    /// The run's constants, by number.
    pub(crate) constants: &'a [CellWord],
    /// The word of each kind code.
    pub(crate) kinds: &'a [CellWord],
    /// The word of each field code.
    pub(crate) fields: &'a [CellWord],
}

/// What deciding shapes counted: the candidates read at the entries, and those that passed the entry star's checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShapeCounts {
    /// Elements of the entry kinds read.
    pub(crate) candidates: Count,
    /// Of them, those whose neighbourhood the entry star holds.
    pub(crate) survivors: Count,
}

/// A file's enclosing columns, made on the first ask of each kind: for a kind code, a bit for each node of the own tree
/// that is a node of that kind or lies inside one. One pass over the kind's elements and their subtrees' ends.
#[derive(Debug, Default)]
pub(crate) struct EnclosingColumns {
    /// The column of each kind asked, by kind code.
    pub(crate) columns: hashbrown::HashMap<u16, Vec<u64>>,
}
