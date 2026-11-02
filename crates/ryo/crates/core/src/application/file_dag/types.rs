use crate::domain::file_program::types::RequestGraph;
use crate::domain::table_model::types::Table;
use std::sync::Arc;

/// Application owns one immutable physical file generation's execution state.
pub(crate) struct FileDagExecution {
    pub graph: Arc<RequestGraph>,
    /// A complete rule is admitted as a unit; partial rules use the residual executor.
    pub active: Vec<bool>,
    /// Root and dependency consumers, not occurrences in individual rule plans.
    pub remaining: Vec<usize>,
    pub ready: Vec<Option<Arc<Table>>>,
    pub visiting: Vec<bool>,
    pub executions: Vec<u64>,
    pub demands: u64,
    pub hits: u64,
    pub releases: u64,
    pub retained_bytes: usize,
    pub peak_bytes: usize,
    pub limit: usize,
    pub unretained: u64,
}
