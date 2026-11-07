//! Request hash behind `DerivedContract::identity_hash`; it must follow `request::identity::types::StaticRequest`.
use crate::domain::derived_views::types::ViewRequest;

/// The hash follows semantic equality, including ordered operands and referenced bindings.
/// Diagnostic comparison origins deliberately do not affect either equality or hashing.
pub(super) fn hash_request<H: std::hash::Hasher>(request: &ViewRequest, state: &mut H) {
    use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral};
    use std::hash::Hash;
    fn operand<H: std::hash::Hasher>(value: &Operand, state: &mut H) {
        std::mem::discriminant(value).hash(state);
        match value {
            Operand::Column(value) => value.hash(state),
            Operand::Parameter(value) => value.hash(state),
            Operand::Literal(QueryLiteral::Number(value)) => value.hash(state),
            Operand::Literal(QueryLiteral::Text(value)) => value.hash(state),
            Operand::Function(value) => {
                value.function.hash(state);
                value.arguments.len().hash(state);
                for value in &value.arguments {
                    operand(value, state);
                }
            }
        }
    }
    fn binding<H: std::hash::Hasher>(
        value: &ryo_values::domain::query_bindings::types::ParameterBinding,
        state: &mut H,
    ) {
        use ryo_values::domain::query_bindings::types::ParameterBinding as B;
        std::mem::discriminant(value).hash(state);
        match value {
            B::Number(value) => value.hash(state),
            B::Flag(value) => value.hash(state),
            B::Text(value) => value.hash(state),
            B::Numbers(value) => value.hash(state),
            B::Texts(value) => value.hash(state),
            B::TextSets(value) => value.hash(state),
            B::Tables(tables) => {
                tables.len().hash(state);
                for table in tables {
                    table.0.len().hash(state);
                    for cell in &table.0 {
                        cell.key.hash(state);
                        cell.value.hash(state);
                    }
                }
            }
        }
    }
    std::mem::discriminant(request).hash(state);
    match request {
        ViewRequest::RawKeyOrder { stride } => stride.0.hash(state),
        ViewRequest::Equality { columns } | ViewRequest::DistinctRepresentatives { columns } => {
            columns.hash(state)
        }
        ViewRequest::PredicateMatches(selector) => {
            selector.relation.hash(state);
            selector.identity.comparisons.len().hash(state);
            for comparison in &selector.identity.comparisons {
                comparison.test.hash(state);
                operand(&comparison.left, state);
                operand(&comparison.right, state);
            }
            selector.identity.bindings.len().hash(state);
            for value in &selector.identity.bindings {
                binding(value, state);
            }
        }
        ViewRequest::CandidatePostings { column, literals } => {
            column.hash(state);
            literals.hash(state);
        }
        ViewRequest::Adjacency { endpoints, keys } => {
            endpoints.hash(state);
            keys.hash(state);
        }
        ViewRequest::Membership { column } => column.hash(state),
        ViewRequest::PartitionOrder { keys } => keys.hash(state),
        ViewRequest::ClosureAdjacency {
            source,
            destination,
        }
        | ViewRequest::CycleAdjacency {
            source,
            destination,
        } => {
            source.hash(state);
            destination.hash(state);
        }
        ViewRequest::OrderedKeys {
            left_on,
            right_on,
            left_order,
            right_order,
        } => {
            left_on.hash(state);
            right_on.hash(state);
            left_order.hash(state);
            right_order.hash(state);
        }
        ViewRequest::OrderedDfs(value) => {
            value.root.hash(state);
            value.source.hash(state);
            value.destination.hash(state);
            value.order.hash(state);
            value.edge.hash(state);
            value.root_keys.hash(state);
            value.edge_keys.hash(state);
        }
        ViewRequest::NativeLookup {
            left_on,
            right_on,
            view,
        } => {
            left_on.hash(state);
            right_on.hash(state);
            view.hash(state);
        }
    }
}
