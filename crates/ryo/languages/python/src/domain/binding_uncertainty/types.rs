//! The one request shape of the `binding_uncertainty` concept.

use crate::domain::{
    binding_results::types::{Candidate, ReadState},
    model::types::{BindingMutation, Declarations},
};
use ryo::languages::{Position, Symbol, Tree};

/// Read-site state and mutable candidate destination for one uncertainty refinement.
pub(crate) struct UncertaintyRefinement<'tree, 'source> {
    /// Parsed source and native syntax relationships.
    pub tree: &'tree Tree<'source>,
    /// File-local indexes used to identify lexical scopes.
    pub model: &'tree Declarations,
    /// The reference occurrence being refined.
    pub reference: Position,
    /// Evaluation phase containing the read.
    pub phase: Position,
    /// Normalized name being read.
    pub name: Symbol,
    /// Result of resolving against the scheduled environment.
    pub state: ReadState,
    /// Ordered mutations in this file.
    pub mutations: &'tree [BindingMutation],
    /// Candidate declarations to extend when control flow may repeat.
    pub candidates: &'tree mut Vec<Candidate>,
}
