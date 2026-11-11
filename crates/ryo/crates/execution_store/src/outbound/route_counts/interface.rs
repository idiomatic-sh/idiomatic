//! What route accounting asks of a format: the document of one route record, written in the format's own syntax.
use crate::outbound::route_counts::types::{RouteDocument, RouteValue};

/// What every `route_counts` adapter implements.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
/// use ryo_execution_store::outbound::route_counts::types::RouteValue;
/// fn tally(format: &dyn RouteCountsPort) -> String {
///     format.document(&RouteValue::Tally(3)).0
/// }
/// ```
pub trait RouteCountsPort: Send + Sync + std::fmt::Debug {
    /// The document of one route record.
    fn document(&self, record: &RouteValue) -> RouteDocument;
}
