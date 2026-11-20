//! Values of the lookup path: plain data, public fields, no constructor.
use crate::domain::binding_resolution::types::TypeParameter;
use crate::domain::binding_results::types::ReadContext;
use crate::domain::model::types::BindingAnalysisState;
use ryo::languages::{Integer, Position, Symbol};

/// One namespace on one reference's lookup path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathStep {
    /// The read or deletion occurrence.
    pub reference: Position,
    /// The name read or deleted.
    pub name: Symbol,
    /// The namespace; a type parameter's defines id for its scope.
    pub scope: Position,
    /// The namespace's place on the path: 0 for the first.
    pub depth: Integer,
    /// The reference's value, type or deletion context.
    pub context: ReadContext,
}

/// Capacity-retaining lookup paths of the current file, made on the first read of the relation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LookupScopeScratch {
    /// Rows by the reference's place in the schedule, then depth.
    pub rows: Vec<PathStep>,
    /// Whether the current file's paths have been made.
    pub(super) state: BindingAnalysisState,
    /// The file's type parameters.
    pub(super) parameters: Vec<TypeParameter>,
}
