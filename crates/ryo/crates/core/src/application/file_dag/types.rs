use super::relations::{RelationOwner, RelationResult};
use crate::domain::file_program::types::RequestGraph;
use std::rc::Rc;
use std::sync::Arc;

/// Application owns one immutable physical file generation's execution state.
pub(crate) struct FileDagExecution {
    /// Actual named-root pins on this file generation, absent for ordinary requests.
    pub(super) observations: Option<super::observations::FileObservations>,
    pub graph: Arc<RequestGraph>,
    pub(super) relations: Rc<RelationOwner>,
    /// A complete rule is admitted as a unit; partial rules use the residual executor.
    pub active: Vec<bool>,
    /// Root and dependency consumers, not occurrences in individual rule plans.
    pub remaining: Vec<usize>,
    /// Optional cross-rule retained results, subject to the existing cache allowance.
    pub(super) ready: Vec<Option<crate::application::file_dag::relations::RetainedPublication>>,
    /// Required live results for the current rule, independent of that allowance.
    pub(super) required: Vec<Option<crate::application::file_dag::relations::PublishedInput>>,
    pub required_remaining: Vec<usize>,
    pub required_executed: Vec<bool>,
    pub required_inputs: Vec<bool>,
    pub required_nodes: Vec<crate::domain::file_program::types::NodeId>,
    pub current_rule: Option<usize>,
    pub required_bytes: usize,
    pub peak_required_bytes: usize,
    /// Shared Arc payload held by both owners; subtract it for unique owned bytes.
    pub overlap_bytes: usize,
    pub peak_owned_bytes: usize,
    pub required_releases: u64,
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
