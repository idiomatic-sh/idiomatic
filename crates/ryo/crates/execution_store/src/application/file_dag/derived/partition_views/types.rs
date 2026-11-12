//! The leases a partitioned closure reads, held for its cursor by the derived concept.
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;

/// The leases a partitioned closure reads: the shared ordering, and the partition the cursor is on.
pub(crate) struct PartitionViews {
    pub(crate) ordered: AcquiredView,
    pub(crate) current: Option<AcquiredView>,
}
