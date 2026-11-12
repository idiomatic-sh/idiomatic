//! One prepared shared ordering and one reusable partition-local adjacency: the cursor over a publication's partitions.
//! Its views are admitted, built and held by the derived concept (`derived::partition_views`).
use super::types::OrderCursor;
use crate::application::file_dag::{
    derived::partition_views::types::PartitionViews, relations::service::RelationOwner,
};
use ryo_execution_domain::domain::derived_views::types::DerivedScratch;
use ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;
use std::{rc::Rc, sync::Arc};

/// A partitioned closure over a publication, for the proofs of its cursor.
///
/// # Errors
///
/// The input's refusal to validate, and the owner's refusal to admit the publication or to prepare and acquire its
/// shared ordering.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "fixtures")]
/// # fn shown() {
/// use ryo_execution_store::application::file_dag::closure_partitions::service::prepare;
/// let _prepare = prepare;
/// # }
/// ```
#[cfg(any(test, feature = "fixtures"))]
pub fn prepare<'f>(
    publication: &crate::application::file_dag::relations::handle::service::Publication,
    input: RepresentationInput<'f>,
    keys: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    source: ryo_values::domain::plan_parts::types::ColumnIndex,
    destination: ryo_values::domain::plan_parts::types::ColumnIndex,
    owner: Rc<RelationOwner>,
) -> Result<PartitionedClosure<'f>, OperatingError> {
    let relations = Rc::clone(&owner);
    relations
        .derived()
        .partitioned_closure(publication, input, keys, source, destination, owner)
}

/// Ordering metadata can be shared across consumers; only the current partition
/// owns values/adjacency. Advancing releases the previous partition's logical contents.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
/// fn named(_: Option<&PartitionedClosure<'_>>) {}
/// ```
pub struct PartitionedClosure<'f> {
    pub(crate) input: RepresentationInput<'f>,
    pub(crate) proof: crate::application::file_dag::relations::admission::service::BorrowedInput<'f>,
    pub(crate) owner: Rc<RelationOwner>,
    pub(crate) partition_plan: Arc<DerivedContract>,
    pub(crate) closure_plan: Arc<DerivedContract>,
    /// The shared ordering and the current partition, held for the cursor by the derived owner's concept.
    pub(crate) views: PartitionViews,
    #[cfg(test)]
    pub(crate) ordering_capacity: crate::application::file_dag::derived::owner::acquisition::admission::types::PartitionCapacity,
    pub(crate) cursor: OrderCursor,
    pub(crate) key: Vec<Value>,
    pub(crate) scratch: DerivedScratch,
}
