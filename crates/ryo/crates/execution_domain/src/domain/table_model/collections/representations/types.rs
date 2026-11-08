//! Borrowed representation data; admission and retained ownership live in the engine.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;

use ryo_relations::domain::relations::types::{CellKind, Value};
/// The kind declaration is also the complete proof inventory; dispatch and
/// real fixtures match it exhaustively, with no separately maintained ALL list.
macro_rules! kinds {
    ($($(#[$meta:meta])* $kind:ident),+ $(,)?) => {
        /// The physical representations a relation's rows can have.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RepresentationKind { $($(#[$meta])* $kind,)+ }
        #[cfg(test)]
        impl RepresentationKind {
            pub(crate) const ALL: &'static [Self] = &[$(Self::$kind,)+];
        }
    };
}
kinds!(
    /// Stored columns.
    Stored,
    /// The rows of a syntax tree.
    Tree,
    /// A computed table.
    Computed,
    /// A compact table of packed words.
    Compact,
);
/// Whether a representation can be read in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalShape {
    /// It can be read in full.
    Complete,
    /// Only its readable physical rows can be read.
    ReadablePhysicalFallback,
}
/// What a representation declares of its rows: the input, its kind, its cell kinds and its row count.
pub struct RelationContract<'a> {
    /// The rows' representation.
    pub input: RepresentationInput<'a>,
    /// The kind of the representation.
    pub kind: RepresentationKind,
    /// The cell kinds of its columns.
    pub kinds: &'a [CellKind],
    /// How many rows it holds.
    pub rows: RowCount,
    /// Captured metadata refusals are interpreted only at the existing demand gate.
    pub metadata: Result<(), OperatingError>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellRead {
    Value(Value),
    MissingPhysical,
}
pub(crate) enum RowRead<'a> {
    Borrowed(&'a [Value]),
    Decoded(&'a [Value]),
    MissingPhysical,
}

use ryo_values::domain::rows::types::RowCount;
