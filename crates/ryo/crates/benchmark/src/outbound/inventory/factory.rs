//! Builds the inventory adapters, one function each, by constructor injection: a factory takes
//! the clients and settings the edge opened and returns the adapter built with them, so no
//! port method takes configuration. A factory builds nothing else, chooses nothing, and is
//! called only by an edge.

/// The local filesystem inventory adapter.
#[must_use]
pub fn local() -> impl crate::outbound::inventory::interface::InventoryPort {
    super::adapters::local::adapter::InventoryLocalAdapter
}
