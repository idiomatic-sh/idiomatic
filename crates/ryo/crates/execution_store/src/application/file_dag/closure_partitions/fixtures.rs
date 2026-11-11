//! The working capacity of the partitioned closure cursor, which its proofs and the engine's observations read.
use super::constants::LIVE_PARTITION_VIEW;
use super::service::PartitionedClosure;

/// Reserved Value buffers; shared ordering remains charged independently.
///
/// # Panics
///
/// When the cursor holds a partition whose view cannot be read as a closure adjacency.
#[expect(
    clippy::expect_used,
    reason = "a fixture: the cursor it measures holds a live partition, and a broken one fails the proof that reads it"
)]
pub fn working_capacity(closure: &PartitionedClosure<'_>) -> usize {
    closure.current.as_ref().map_or(0, |_| {
        let current = closure.current().expect(LIVE_PARTITION_VIEW);
        let capacities = ryo_derived::domain::closure_adjacency::service::capacities(current);
        current.table.cells.capacity() + capacities.key.0 + capacities.sources.0
    }) + closure.key.capacity()
        + closure.scratch.row.capacity()
}
