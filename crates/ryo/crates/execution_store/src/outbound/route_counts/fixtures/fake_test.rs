//! The `route_counts` port's fake runs the port's contract.

use super::super::contract::check;
use super::RouteCountsFake;

#[test]
fn succeeds() {
    check(&RouteCountsFake);
}
