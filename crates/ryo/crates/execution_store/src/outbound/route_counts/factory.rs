//! Builds the route_counts adapters, one function each, by constructor injection: a factory takes
//! the clients and settings the edge opened and returns the adapter built with them, so no
//! port method takes configuration. A factory builds nothing else, chooses nothing, and is
//! called only by an edge.

/// The json route_counts adapter, built from the clients the edge passes.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::outbound::route_counts::factory::json;
/// use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort as _;
/// use ryo_execution_store::outbound::route_counts::types::RouteValue;
/// assert_eq!(json().document(&RouteValue::Count(3)).0, "3");
/// ```
#[must_use]
pub fn json() -> impl crate::outbound::route_counts::interface::RouteCountsPort {
    super::adapters::json::adapter::RouteCountsJsonAdapter
}
