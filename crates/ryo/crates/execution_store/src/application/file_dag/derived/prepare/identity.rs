//! Request hash behind `DerivedContract::identity_hash`; it must follow `request::identity::types::StaticRequest`.
use crate::domain::derived_views::types::ViewRequest;

/// The hash follows semantic equality.
pub(super) fn hash_request<H: std::hash::Hasher>(request: &ViewRequest, state: &mut H) {
    use std::hash::Hash;
    std::mem::discriminant(request).hash(state);
    match request {
        ViewRequest::Equality { columns } => columns.hash(state),
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
