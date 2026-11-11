//! The fake of the `route_counts` port: what the core's tests stand in for it with.

use super::super::interface::RouteCountsPort;
use crate::outbound::route_counts::types::{RouteDocument, RouteValue};

/// What the core's tests stand in for `route_counts` with: it writes a record as its debug text.
#[derive(Debug)]
pub struct RouteCountsFake;

impl RouteCountsPort for RouteCountsFake {
    fn document(&self, record: &RouteValue) -> RouteDocument {
        RouteDocument(format!("{record:?}"))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
