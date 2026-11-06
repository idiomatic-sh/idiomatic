//! Facts the admitted view reports about the storage it leases.

/// Reserved room of a partition order, in row positions.
pub(in crate::application::file_dag) struct PartitionCapacity(pub usize);

/// Which allocation a leased view reads, for proving two views share it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct StorageIdentity(
    pub *const crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage,
);
