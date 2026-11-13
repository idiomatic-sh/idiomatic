//! Static reference classification; definite absence, possible absence and unsupported knowledge are distinct.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// Static reference classification; definite absence, possible absence and unsupported knowledge are distinct.
#[derive(Clone, Copy, Debug)]
pub struct BindingRead;
impl super::super::interface::sealed::Sealed for BindingRead {}
/// Declare the reusable static binding fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_read {
    ($callback:path) => {
        $callback! {
        item BindingRead,
        row { /// One explicit binding-analysis observation.
        BindingReadRow },
        column { /// Public fact columns.
        BindingReadColumn },
        expected { /// Independently specified observation.
        BindingReadExpectedRow },
        class Code,
        columns {
        /// Real semantic read, write or deletion occurrence.
        reference Reference: NodeId => Node;
        /// Canonical lexical identifier.
        name Name: Symbol => Symbol;
        /// Lexical scope of the reference.
        scope Scope: NodeId => Node;
        /// Execution boundary used by static association.
        phase Phase: NodeId => Node;
        /// Value, type, write, read-write or deletion context; never a lint policy.
        context Context: Symbol => Symbol;
        /// Unique declaration, alternative declarations, possibly-unbound, unbound, builtin, declaration-or-builtin, or uncertain. These are static associations, not runtime initialization assertions.
        state State: Symbol => Symbol;
        /// First major language version supplying a statically associated builtin; applicable only when state is builtin or declaration-or-builtin. Zero otherwise is inapplicable, not an availability claim.
        since_major SinceMajor: Integer => Number;
        /// First minor language version supplying that builtin; applicable only when state is builtin or declaration-or-builtin. Target-version policy belongs to the consumer.
        since_minor SinceMinor: Integer => Number;
        /// Source file defining these identities.
        file File: FileId => File;
        /// Original occurrence extent, independent of evaluation order.
        span Span: Span => Span;
        }
        }
    };
}
crate::binding_read!(crate::relation_row);
