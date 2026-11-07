//! One row for each vertex a walk reaches from each source, partition by partition, in walk order.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::evaluation::types::{
    EvaluationScratch, InvocationDriver, PartitionPlace, VertexPlace,
};

/// Emits, for each partition of the engine's closure and each of its sources, the partition's key, the
/// source and each vertex the source reaches.
pub(crate) fn evaluate(
    driver: &mut dyn InvocationDriver,
    scratch: &mut EvaluationScratch,
) -> Result<(), PortableOperatorFault> {
    let EvaluationScratch {
        row: joined,
        partition: key,
        ..
    } = scratch;
    let partitions = driver.closure().partitions()?;
    for place in (0..partitions.0).map(PartitionPlace) {
        driver.closure().partition_key(place, key)?;
        let sources = driver.closure().sources(place);
        for at in (0..sources.0).map(VertexPlace) {
            let start = driver
                .closure()
                .source(place, at)
                .ok_or(PortableOperatorFault::ShapeMismatch)?;
            let reached = driver.closure().reach(place, start)?;
            for index in (0..reached.0).map(VertexPlace) {
                let vertex = driver
                    .closure()
                    .reached(index)
                    .ok_or(PortableOperatorFault::ShapeMismatch)?;
                joined.clear();
                joined.extend_from_slice(key);
                joined.extend_from_slice(&[start, vertex]);
                driver.emit(joined)?;
            }
        }
    }
    Ok(())
}
