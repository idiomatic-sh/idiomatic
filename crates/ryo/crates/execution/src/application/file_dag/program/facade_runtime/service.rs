//! Opening one publication scope and naming the physical files of one project request.
use super::types::{ProgramServices, PublicationScope, ReuseEligibility};
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::scope::open_program_owner;
use crate::domain::coverage::types::{Covered, RuleIndex};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::runs::types::Skipped;
use std::sync::Arc;

pub(crate) fn publication_owner(
    program: &Arc<PreparedProgram>,
    services: &ProgramServices,
) -> Result<PublicationScope, OperatingError> {
    Ok(PublicationScope {
        request: Arc::clone(&services.request),
        program: Arc::clone(program.program()),
        owner: open_program_owner(program.program(), services.derived().clone(), ReuseEligibility::Eligible),
        execution: std::cell::OnceCell::new(),
        walk:
            crate::application::file_dag::program::native_walk::project::service::ProjectWalk::new(),
        populations: std::cell::RefCell::new(Default::default()),
        symbols: std::cell::OnceCell::new(),
    })
}

/// The finite physical files named by this project request. No absent frame is
/// synthesized from a missing index, and a skipped file is never complete-empty.
pub(crate) fn required_partitions(
    program: &Arc<PreparedProgram>,
    rule: RuleIndex,
    files: &[ryo_values::domain::files::types::CorpusInput],
    covered: &[Covered],
    skipped: &[Skipped],
    inapplicable: &[ryo_relations::domain::relations::types::FileId],
    services: &ProgramServices,
) -> Result<RequiredPartitions, OperatingError> {
    program.rule_metadata(rule)?;
    RequiredPartitions::from_request(&services.request, files, covered, skipped, inapplicable)
}
