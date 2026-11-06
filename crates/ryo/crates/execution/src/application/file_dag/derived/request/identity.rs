//! Exact typed static request identity; evaluated only during request preparation.
use super::*;
pub(in crate::application::file_dag::derived) fn same_request(
    left: &ViewRequest,
    right: &ViewRequest,
) -> bool {
    match (left, right) {
        (ViewRequest::RawKeyOrder { stride: left }, ViewRequest::RawKeyOrder { stride: right }) => {
            left == right
        }
        (ViewRequest::Equality { columns: a }, ViewRequest::Equality { columns: b })
        | (
            ViewRequest::DistinctRepresentatives { columns: a },
            ViewRequest::DistinctRepresentatives { columns: b },
        ) => a == b,
        (ViewRequest::PredicateMatches(a), ViewRequest::PredicateMatches(b)) => {
            a.relation == b.relation && a.identity == b.identity
        }
        (
            ViewRequest::CandidatePostings {
                column: a,
                literals: x,
            },
            ViewRequest::CandidatePostings {
                column: b,
                literals: y,
            },
        ) => a == b && x == y,
        (
            ViewRequest::Adjacency {
                endpoints: a,
                keys: x,
            },
            ViewRequest::Adjacency {
                endpoints: b,
                keys: y,
            },
        ) => a == b && x == y,
        (ViewRequest::Membership { column: a }, ViewRequest::Membership { column: b }) => a == b,
        (ViewRequest::PartitionOrder { keys: a }, ViewRequest::PartitionOrder { keys: b }) => {
            a == b
        }
        (
            ViewRequest::ClosureAdjacency {
                source: a,
                destination: x,
            },
            ViewRequest::ClosureAdjacency {
                source: b,
                destination: y,
            },
        )
        | (
            ViewRequest::CycleAdjacency {
                source: a,
                destination: x,
            },
            ViewRequest::CycleAdjacency {
                source: b,
                destination: y,
            },
        ) => a == b && x == y,
        (
            ViewRequest::OrderedKeys {
                left_on: a,
                right_on: b,
                left_order: c,
                right_order: d,
            },
            ViewRequest::OrderedKeys {
                left_on: w,
                right_on: x,
                left_order: y,
                right_order: z,
            },
        ) => a == w && b == x && c == y && d == z,
        (ViewRequest::OrderedDfs(a), ViewRequest::OrderedDfs(b)) => {
            a.root == b.root
                && a.source == b.source
                && a.destination == b.destination
                && a.order == b.order
                && a.edge == b.edge
                && a.root_keys == b.root_keys
                && a.edge_keys == b.edge_keys
        }
        (
            ViewRequest::NativeLookup {
                left_on: a,
                right_on: b,
                view: c,
            },
            ViewRequest::NativeLookup {
                left_on: x,
                right_on: y,
                view: z,
            },
        ) => a == x && b == y && c == z,
        _ => false,
    }
}

/// The hash follows semantic equality, including ordered operands and referenced bindings.
/// Diagnostic comparison origins deliberately do not affect either equality or hashing.
pub(in crate::application::file_dag::derived) fn hash_request<H: std::hash::Hasher>(
    request: &ViewRequest,
    state: &mut H,
) {
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
        ViewRequest::RawKeyOrder { stride } => stride.hash(state),
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
