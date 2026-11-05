//! Build the stateless JSON collector codec.
/// Native argument and resource wire protocol.
#[must_use]
pub fn json() -> impl super::interface::CollectionPort {
    super::adapters::json::adapter::CollectionJsonAdapter
}
