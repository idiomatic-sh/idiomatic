//! Structural source and invocation records have no facade or evaluation dependency.
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_execution_domain::domain::{
    completion::types::{CoverageState, completion_policy::value::CompletionPolicy},
    executable_program::types::InputSource,
    operating::errors::OperatingError,
};
use ryo_values::domain::{plan_parts::types::Input, rows::types::Table};
use std::{cell::OnceCell, sync::Arc};
#[derive(Clone, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum ProjectFrontierPayload {
    Flat(Arc<Table>),
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
}
/// The ID and backing are inseparable and privately issued by this phase owner.
pub(in crate::application::file_dag::program) struct ProjectFrontierPopulation<A> {
    pub(in crate::application::file_dag::program) id: PopulationId,
    pub(in crate::application::file_dag::program) source: Result<InputSource, OperatingError>,
    pub(in crate::application::file_dag::program) coverage: CoverageState,
    pub(in crate::application::file_dag::program) payload: ProjectFrontierPayload,
    pub(in crate::application::file_dag::program) admitted: OnceCell<Result<A, OperatingError>>,
}
#[derive(PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct ProjectInvocationKey<R> {
    pub(in crate::application::file_dag::program) sources: Vec<PopulationId>,
    pub(in crate::application::file_dag::program) refused: Vec<ProjectRefusal>,
    pub(in crate::application::file_dag::program) policy: CompletionPolicy,
    pub(in crate::application::file_dag::program) bound: Vec<BoundKey<R>>,
}
#[derive(PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct ProjectRefusal {
    pub(in crate::application::file_dag::program) source:
        Option<Result<InputSource, OperatingError>>,
    pub(in crate::application::file_dag::program) input: Input,
    pub(in crate::application::file_dag::program) error: OperatingError,
}
#[derive(PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum BoundKey<R> {
    Current(PopulationId, CoverageState),
    Restored(R),
    Refused(OperatingError),
    Input(ryo_execution_domain::domain::executable_program::types::InputSource),
}

/// Structural equality is the only reuse decision accepted by the source interner.
pub(in crate::application::file_dag::program) enum ProjectSourceMatch {
    Equivalent,
    Distinct,
}
