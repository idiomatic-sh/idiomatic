//! Per-row and request-owned routes beside a shared head: barrier, classification, isolation.
use super::super::fixtures::routes::RowRoute;
use super::fixtures::exercise;

#[test]
fn per_row_and_request_owned_routes_are_barriers_beside_a_shared_head() {
    for route in RowRoute::ALL {
        exercise(route).unwrap_or_else(|error| panic!("{route:?}: {error:?}"));
    }
}
