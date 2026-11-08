//! Prepared frontiers a test hands to a project phase without reading a provider: the fields stay private to `frontiers`.
use super::types::{FrontierInput, FrontierSource, PreparedFrontiers};
use std::sync::Arc;

/// The prepared frontiers of `producer`, as a preparation would hand them over.
pub(in crate::application::file_dag) fn prepared_frontiers(
    producer: crate::domain::coverage::types::RuleIndex,
    sources: Arc<[FrontierSource]>,
    rows: Arc<[FrontierInput]>,
    program: Arc<crate::domain::executable_program::types::executable_program::value::ExecutableProgram>,
    required: crate::application::file_dag::program::admission::RequiredPartitions,
    refused: Arc<[crate::domain::frontiers::types::RefusedFrontier]>,
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
