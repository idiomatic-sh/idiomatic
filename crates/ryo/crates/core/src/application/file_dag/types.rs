use super::relations::RelationOwner;
use crate::domain::file_program::types::RequestGraph;
use std::rc::Rc;
use std::sync::Arc;

/// Application owns one immutable physical file generation's execution state.
pub(crate) struct FileDagExecution {
    pub(super) failed: Vec<Option<crate::domain::operating::errors::OperatingError>>,
    pub(super) counting: crate::domain::lowering::types::Counting,
    pub(super) fed: Vec<crate::domain::operator::types::RowsFed>,
    pub(super) fed_rule: Vec<crate::domain::operator::types::RowsFed>,
    pub(super) fed_touched: Vec<crate::domain::file_program::types::NodeId>,
    pub(super) native_steps: crate::domain::operator::types::TreeSteps,
    pub(super) kernel_counts: crate::domain::column_kernel::types::KernelCounts,
    pub(super) kernel_strategy: crate::domain::column_kernel::types::KernelStrategy,
    pub(super) classes: std::sync::Arc<crate::domain::shared_walk::types::SharedClasses>,
    pub(super) class_of: Vec<Vec<Option<crate::domain::positions::types::ClassPlace>>>,
    pub(super) coverage: Vec<Option<super::completion::CoverageState>>,
    pub(super) completion: super::completion::CompletionPolicy,
    pub(super) rooms: std::rc::Rc<super::invocation::rooms::InvocationRooms>,
    /// Actual named-root pins on this file generation, absent for ordinary requests.
    pub(super) observations: Option<super::observations::FileObservations>,
    pub graph: Arc<RequestGraph>,
    pub(super) phase: ryo_values::domain::plans::types::StepScope,
    /// Phase-boundary publications/failures belong to this same owner. A
    /// missing producer is never evaluated in the consumer's phase/context.
    pub(super) provided:
        std::collections::HashMap<crate::domain::file_program::types::NodeId, ProvidedInput>,
    pub(super) provided_decodes: u64,
    pub(super) provided_releases: u64,
    pub(super) provided_peak_cells: usize,
    pub(super) provided_allocations:
        Vec<(std::sync::Weak<super::relations::RelationResult>, usize)>,
    pub(super) provided_observations: std::collections::HashMap<
        crate::domain::file_program::types::ObservationGraphInputId,
        Result<super::relations::PublishedInput, crate::domain::operating::errors::OperatingError>,
    >,
    pub(super) hosted: std::collections::HashMap<
        crate::domain::file_program::types::NodeId,
        super::relations::PublishedInput,
    >,
    pub(super) relations: Rc<RelationOwner>,
    /// Applicable rules use this scheduler regardless of completeness.
    pub active: Vec<bool>,
    pub(super) reusable: Vec<bool>,
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

/// Supplied phase producers never fall back to execution in another phase.
#[derive(Clone)]
pub(super) enum ProvidedInput {
    Compact {
        input: ryo_values::domain::plan_parts::types::Input,
        table: std::sync::Arc<crate::domain::table_words::types::CompactTable>,
    },
    Live {
        input: ryo_values::domain::plan_parts::types::Input,
        publication: super::relations::PublishedInput,
    },
    Refused {
        input: ryo_values::domain::plan_parts::types::Input,
        error: crate::domain::operating::errors::OperatingError,
    },
    Materializing {
        input: ryo_values::domain::plan_parts::types::Input,
    },
    Retired {
        input: ryo_values::domain::plan_parts::types::Input,
    },
}
