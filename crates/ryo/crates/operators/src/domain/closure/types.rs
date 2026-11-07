//! What a closure reads from the engine: its partitions, their sources and the walk over them.

/// A partition of a closure, by its place among the closure's partitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionPlace(pub usize);

/// The engine's borrowed closure partitions and its reachability walk over them.
pub trait ClosureDriver {
    /// How many partitions the closure holds.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring the partitions.
    fn partitions(
        &mut self,
    ) -> Result<
        ryo_relations::domain::relations::types::Count,
        crate::domain::contract::types::PortableOperatorFault,
    >;
    /// Copies the key of a partition into `key`.
    ///
    /// # Errors
    ///
    /// The engine retained an error reading the partition.
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> Result<(), crate::domain::contract::types::PortableOperatorFault>;
    /// How many sources a partition has.
    fn sources(
        &self,
        partition: PartitionPlace,
    ) -> ryo_relations::domain::relations::types::Count;
    /// One source of a partition; none when the partition has no source at that place.
    ///
    /// # Errors
    ///
    /// The engine retained an error reading the partition.
    fn source(
        &mut self,
        partition: PartitionPlace,
        at: crate::domain::contract::types::VertexPlace,
    ) -> Result<
        Option<ryo_relations::domain::relations::types::Value>,
        crate::domain::contract::types::PortableOperatorFault,
    >;
    /// Walks every vertex reachable from a source of a partition, in the walk's order.
    ///
    /// # Errors
    ///
    /// The engine retained an error walking the partition.
    fn reach(
        &mut self,
        partition: PartitionPlace,
        source: ryo_relations::domain::relations::types::Value,
    ) -> Result<
        ryo_relations::domain::relations::types::Count,
        crate::domain::contract::types::PortableOperatorFault,
    >;
    /// One vertex the last walk reached.
    fn reached(
        &self,
        at: crate::domain::contract::types::VertexPlace,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
}

/// An engine driver that lends its closure partitions and walk.
pub trait ClosureEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's closure partitions and walk.
    fn closure(&mut self) -> &mut dyn ClosureDriver;
}
