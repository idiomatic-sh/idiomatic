//! The `identity` application operation.

use super::types::StaticRequest;
use ryo_execution_domain::domain::derived_views::types::ViewRequest;

impl PartialEq for StaticRequest<'_> {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = (self.0, other.0);
        match (left, right) {
            (ViewRequest::Equality { columns: a }, ViewRequest::Equality { columns: b }) => a == b,
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
            (ViewRequest::Membership { column: a }, ViewRequest::Membership { column: b }) => {
                a == b
            }
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
}
