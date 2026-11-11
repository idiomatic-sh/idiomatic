//! The types of the `source_admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::admission::service::AvailableInput;
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::admission::service::Representation;
use crate::application::file_dag::program::admission::types::Emptiness;
use crate::application::file_dag::program::admission::types::Partition;
use crate::application::file_dag::program::admission::types::ProducerRoot;
use crate::application::file_dag::program::constants::PRODUCER_COVERAGE_INCOMPLETE;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::facade_runtime::types::CachedInput;
use crate::application::file_dag::program::facade_runtime::types::FileRoot;
use crate::application::file_dag::program::facade_runtime::types::ObservationKey;
use crate::application::file_dag::program::facade_runtime::types::Provision;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::table_model::service::count_of;
use ryo_execution_domain::domain::table_model::service::materialize;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::Table;
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
