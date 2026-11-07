//! Whether a handle's graph is the file cache's retained allocation.

/// Whether the graph behind a handle is retained by the cache for other demands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum GraphSharing {
    Shared,
    Local,
}

/// Which allocation a root answer reads, for proving two answers share it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct DataIdentity(pub *const ());
