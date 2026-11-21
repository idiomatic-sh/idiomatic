//! The types of the `walking` part of the `code` concept: the operands of the words that walk the tree and the edges a body yields, each as the
//! encoding writes them and the executor reads them. A field is a slot of the word under its own name and type.

use crate::domain::code::types::{PoolAt, WordFlag};
use crate::domain::steps::types::Register;
use ryo_language::domain::node_table::types::FieldCode;

/// The operands of `Children`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChildrenWord {
    /// `OWN_TREE` when the walk is of the file's own tree alone.
    pub(crate) flag: WordFlag,
    /// The register holding the parent.
    pub(crate) parent: Register,
    /// The register each child is written to.
    pub(crate) out: Register,
}

/// The operands of `ChildrenOfField`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChildrenOfFieldWord {
    /// `OWN_TREE` when the walk is of the file's own tree alone.
    pub(crate) flag: WordFlag,
    /// The field.
    pub(crate) field: FieldCode,
    /// The register holding the parent.
    pub(crate) parent: Register,
    /// The register each child is written to.
    pub(crate) out: Register,
}

/// The operands of `Axis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AxisWord {
    /// `OWN_TREE` when the walk is of the file's own tree alone.
    pub(crate) flag: WordFlag,
    /// The direction's code.
    pub(crate) direction: WordFlag,
    /// The register holding the start.
    pub(crate) start: Register,
    /// The register each element reached is written to.
    pub(crate) out: Register,
}

/// The operands of `Within`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WithinWord {
    /// `OWN_TREE` when the walk is of the file's own tree alone.
    pub(crate) flag: WordFlag,
    /// The register holding the ancestor.
    pub(crate) ancestor: Register,
    /// The register holding the descendant.
    pub(crate) descendant: Register,
}

/// The operands of `Nearest` and `NearestDerived`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NearestWord {
    /// `KEPT` when the answer is kept per start; over the tree, `OWN_TREE` when the walk is of the file's own tree alone.
    pub(crate) flag: WordFlag,
    /// The register holding the start.
    pub(crate) start: Register,
    /// The register the nearest element is written to.
    pub(crate) out: Register,
    /// Over derived edges the edges' body record and the register the element whose edges are asked is in; then the condition's body record, the register the element under test is in, and the memo site.
    pub(crate) operands: PoolAt,
}

/// The operands of `Closure`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClosureWord {
    /// The register holding the seed.
    pub(crate) seed: Register,
    /// The register each element reached is written to.
    pub(crate) out: Register,
    /// The edges' body record, then the register the element whose edges are asked is in.
    pub(crate) operands: PoolAt,
}
