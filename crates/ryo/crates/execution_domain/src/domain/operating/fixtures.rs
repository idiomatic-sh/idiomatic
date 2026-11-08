//! Test support of the `operating` concept: what a run observed of the engine-owned resources it used.

/// Test-only observations captured from real engine-owned resources before release.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutionObservation {
    /// Calls to the real registered operator dispatch.
    pub registered_calls: usize,
    pub regexes: usize,
    pub globs: usize,
    pub owners_initialized: usize,
    pub row_capacity: usize,
    pub other_capacity: usize,
    pub index_rows: usize,
    /// Actual borrowed representation behind the acquired equality resource.
    pub index_representation:
        Option<crate::domain::table_model::types::RepresentationKind>,
    pub index_cells: usize,
    pub index_capacity: usize,
    pub closure_partition_rows: usize,
    pub closure_partition_capacity: usize,
    pub closure_visited: usize,
    pub dfs_expanded: usize,
    pub dfs_examined: usize,
    pub dfs_adjacency_len: usize,
    pub dfs_frames_len: usize,
    pub dfs_visited_len: usize,
    pub traversal_queue_len: usize,
    /// Set only after a real weak file-owner handle is observed dead at file finish.
    pub file_owner_released: bool,
}
