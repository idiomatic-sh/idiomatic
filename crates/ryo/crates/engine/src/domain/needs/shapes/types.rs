//! The types of the `shapes` part: the shapes a rule set's needs ask the file's tree to hold, and a file's answers.

use crate::domain::code::types::WordSlot;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::KindBits;

/// The shapes the needs ask for, numbered as they are met, found once a worker: each shape term's codes in the language's
/// vocabulary of kinds and fields.
#[derive(Debug, Default)]
pub(crate) struct ShapeWants {
    /// Each shape term's place among `wants`, by its parent kind, field and child kind constants.
    pub(crate) terms: hashbrown::HashMap<(WordSlot, WordSlot, WordSlot), u32>,
    /// What each shape term asks, by place.
    pub(crate) wants: Vec<ShapeCodes>,
}

/// One shape in the codes of the node table: each part the codes whose name is the constant's word (none of them when the
/// vocabulary names no such kind or field: no element holds it); none: any.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ShapeCodes {
    /// The parent's kind.
    pub(crate) parent: Option<Box<[KindBits]>>,
    /// The element's field in its parent.
    pub(crate) field: Option<Box<[FieldCode]>>,
    /// The element's kind.
    pub(crate) child: Option<Box<[KindBits]>>,
}

/// What a file's tree holds of the shapes asked, as bits by the shapes' places.
#[derive(Debug, Default)]
pub(crate) struct ShapeFile {
    /// How many of the shapes the bits answer: none at the file's start.
    pub(crate) known: usize,
    /// The shapes the file holds.
    pub(crate) held: Vec<u64>,
}
