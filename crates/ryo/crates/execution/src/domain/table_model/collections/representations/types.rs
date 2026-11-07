//! Borrowed representation data; admission and retained ownership live in the engine.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RepresentationInput;

use ryo_relations::domain::relations::types::{CellKind, Value};
/// The kind declaration is also the complete proof inventory; dispatch and
/// real fixtures match it exhaustively, with no separately maintained ALL list.
macro_rules! kinds {
    ($($kind:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) enum RepresentationKind { $($kind,)+ }
        #[cfg(test)]
        impl RepresentationKind {
            pub(crate) const ALL: &'static [Self] = &[$(Self::$kind,)+];
        }
    };
}
kinds!(Stored, Tree, Computed, Compact);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicalShape {
    Complete,
    ReadablePhysicalFallback,
}
pub(crate) struct RelationContract<'a> {
    pub input: RepresentationInput<'a>,
    pub kind: RepresentationKind,
    pub kinds: &'a [CellKind],
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
