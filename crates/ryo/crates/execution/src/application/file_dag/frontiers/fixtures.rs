//! Prepared frontiers a test hands to a project phase without reading a provider: the fields stay private to `frontiers`.
use super::types::{FrontierInput, FrontierSource, PreparedFrontiers};
use std::sync::Arc;

/// The prepared frontiers of `producer`, as a preparation would hand them over.
pub(crate) fn prepared_frontiers(
    producer: ryo_execution_domain::domain::coverage::types::RuleIndex,
    sources: Arc<[FrontierSource]>,
    rows: Arc<[FrontierInput]>,
    program: Arc<ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram>,
    required: crate::application::file_dag::program::admission::RequiredPartitions,
    refused: Arc<[ryo_execution_domain::domain::frontiers::types::RefusedFrontier]>,
) -> PreparedFrontiers {
    PreparedFrontiers {
        producer,
        sources,
        rows,
        program,
        required,
        refused,
    }
}
