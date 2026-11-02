//! The outer composition chooses filesystem inventory artifacts.
/// Bind registrations and results to the local filesystem adapter.
#[must_use]
pub fn local() -> impl super::interface::InventoryPort {
    super::adapters::local::adapter::InventoryLocalAdapter
}
