//! Known real declaration candidates for a semantic reference; unresolved references have no fake target edge.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// Known real declaration candidates for a semantic reference; unresolved references have no fake target edge.
#[derive(Clone, Copy, Debug)]
pub struct BindingUse;
impl super::super::interface::sealed::Sealed for BindingUse {}
/// Declare the reusable static binding fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_use {
    ($callback:path) => {
        $callback! {
        item BindingUse,
        row { /// One explicit binding-analysis observation.
        BindingUseRow },
        column { /// Public fact columns.
        BindingUseColumn },
        expected { /// Independently specified observation.
        BindingUseExpectedRow },
        class Code,
        columns {
        /// Real semantic read, write or deletion occurrence.
        reference Reference: NodeId => Node;
        /// Actual defines.id candidate; never an unknown-node sentinel or fabricated builtin.
        binding Binding: NodeId => Node;
        /// Lexical reference scope, including references to an outer namespace.
        scope Scope: NodeId => Node;
        /// Execution boundary used by static association.
        phase Phase: NodeId => Node;
        /// Value, type, write, read-write or deletion context.
        context Context: Symbol => Symbol;
        /// Unique or alternative association. Possible absence remains visible in binding-read.state.
        certainty Certainty: Symbol => Symbol;
        /// Source file defining these identities.
        file File: FileId => File;
        /// Original occurrence extent, independent of evaluation order.
        span Span: Span => Span;
        }
        }
    };
}
crate::binding_use!(crate::relation_row);
