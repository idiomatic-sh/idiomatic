//! Ordered static name events within explicit evaluation phases.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// Ordered static name events within explicit evaluation phases.
#[derive(Clone, Copy, Debug)]
pub struct BindingEvent;
impl super::super::interface::sealed::Sealed for BindingEvent {}
/// Declare the reusable static binding fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_event {
    ($callback:path) => {
        $callback! {
        item BindingEvent,
        row { /// One explicit binding-analysis observation.
        BindingEventRow },
        column { /// Public fact columns.
        BindingEventColumn },
        expected { /// Independently specified observation.
        BindingEventExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        columns {
        /// Real syntax occurrence; declaration events reuse defines.id and cleanup uses its handler.
        occurrence Occurrence: NodeId => Node;
        /// Canonical identifier; unknown for namespace-wide events with no single binding name.
        name Name: Symbol => Symbol;
        /// Affected namespace; read events retain their lexical scope.
        scope Scope: NodeId => Node;
        /// Real execution-boundary identity; separate from lexical namespace.
        phase Phase: NodeId => Node;
        /// Language event kind, including read, declaration, deletion and handler cleanup.
        kind Kind: Symbol => Symbol;
        /// File-local static event order; never runtime interleaving or a source byte offset.
        ordinal Ordinal: Integer => Number;
        /// Source file defining these identities.
        file File: FileId => File;
        /// Original occurrence extent, independent of evaluation order.
        span Span: Span => Span;
        }
        }
    };
}
crate::binding_event!(crate::relation_row);
