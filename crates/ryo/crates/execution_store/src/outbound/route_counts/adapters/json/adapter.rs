//! The route_counts port, implemented with json.

use super::writer::write;
use crate::outbound::route_counts::interface::RouteCountsPort;
use crate::outbound::route_counts::types::{RouteDocument, RouteValue};

/// The json route_counts adapter.
#[derive(Debug)]
pub(crate) struct RouteCountsJsonAdapter;

impl RouteCountsPort for RouteCountsJsonAdapter {
    fn document(&self, record: &RouteValue) -> RouteDocument {
        let mut out = RouteDocument::default();
        write(&mut out, record);
        out
    }
}
