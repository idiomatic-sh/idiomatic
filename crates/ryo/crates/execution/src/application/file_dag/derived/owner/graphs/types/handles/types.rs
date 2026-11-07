//! Whether a handle's graph is the file cache's retained allocation.

/// Whether the graph behind a handle is retained by the cache for other demands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum GraphSharing {
    Shared,
    Local,
}

/// The membership marks of a domain over its dense vertex space: the bare marks inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct DomainMarks<'a>(pub &'a [bool]);

/// Which allocation a root answer reads, for proving two answers share it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct DataIdentity(pub *const ());
