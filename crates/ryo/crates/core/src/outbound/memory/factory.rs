//! Builds the memory adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The sysinfo memory adapter, over the most memory the caller lets a run use.
#[must_use]
pub fn sysinfo(
    asked: ryo_values::domain::runs::types::Budget,
) -> impl crate::outbound::memory::interface::MemoryPort {
    super::adapters::sysinfo::adapter::MemorySysinfoAdapter::new(asked)
}
