//! Test support of the `operating` concept: what a run observed of the engine-owned resources it used.

/// Test-only observations captured from real engine-owned resources before release.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutionObservation {
    pub regexes: usize,
    pub globs: usize,
    pub row_capacity: usize,
    pub other_capacity: usize,
    pub index_rows: usize,
    pub closure_visited: usize,
    pub dfs_expanded: usize,
    pub dfs_examined: usize,
    pub dfs_frames_len: usize,
    pub dfs_visited_len: usize,
}
