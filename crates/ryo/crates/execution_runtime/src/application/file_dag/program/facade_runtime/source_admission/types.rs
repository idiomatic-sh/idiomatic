//! The types of the `source_admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::program::admission::service::AvailableInput;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::facade_runtime::types::CachedInput;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use std::rc::Rc;

pub(crate) struct ProjectSources<'a> {
    pub(crate) program: &'a PreparedProgram,
    pub(crate) rule: RuleIndex,
    pub(crate) sources: &'a [crate::application::file_dag::frontiers::types::FrontierSource],
    pub(crate) ordinary: &'a [Rc<
        super::super::super::project_population::types::ProjectFrontierPopulation<AvailableInput>,
    >],
    pub(crate) refused: &'a [ryo_execution_domain::domain::frontiers::types::RefusedFrontier],
    pub(crate) bound:
        &'a mut [Option<crate::application::file_dag::frontiers::types::BoundInput>],
    pub(crate) cache: &'a mut Vec<CachedInput>,
    pub(crate) transport: &'a ryo_language::domain::symbols::types::TransportPermit,
    pub(crate) publication: &'a PublicationScope,
}
