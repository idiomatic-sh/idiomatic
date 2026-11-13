//! Facts the admitted view reports about the storage it leases.

/// Reserved room of a partition order, in row positions.
#[cfg(test)]
pub(crate) struct PartitionCapacity(pub usize);

/// Which allocation a leased view reads, for proving two views share it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StorageIdentity(
    pub *const crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage,
);
