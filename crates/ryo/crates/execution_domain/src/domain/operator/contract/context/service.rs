//! The `context` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::{PrimitiveLease, PrimitiveViews, PrimitiveWorkspace, WorkCounters};
use super::super::views::types::PredicateProgram;
use crate::domain::operating::types::PrimitiveSemantics;
use ryo_language::domain::symbols::types::QuerySymbols;

/// The application invocation bridge owns the only production call site.
pub fn with_primitive_context<'a, T>(
    views: PrimitiveViews<'a>,
    workspace: &'a mut PrimitiveWorkspace,
    derived: QuerySymbols<'a>,
    predicate: &'a mut dyn PredicateProgram,
    semantics: &'a dyn PrimitiveSemantics,
    work: &'a WorkCounters,
    action: impl FnOnce(&mut PrimitiveLease<'a>) -> T,
) -> T {
    let mut context = crate::domain::operator::contract::lease::service::primitive_lease(
        views, workspace, derived, predicate, semantics, work,
    );
    action(&mut context)
}
