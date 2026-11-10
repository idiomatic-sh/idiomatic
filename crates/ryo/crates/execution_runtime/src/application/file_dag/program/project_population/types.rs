//! Structural source and invocation records have no facade or evaluation dependency.
use ryo_execution_domain::domain::{
    completion::types::{CoverageState, completion_policy::value::CompletionPolicy},
    executable_program::types::InputSource,
    operating::errors::OperatingError,
};
use ryo_execution_store::application::file_dag::derived::population::types::PopulationId;
use ryo_values::domain::{plan_parts::types::Input, rows::types::Table};
use std::{cell::OnceCell, sync::Arc};
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ProjectFrontierPayload {
    Flat(Arc<Table>),
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
}
/// The ID and backing are inseparable and privately issued by this phase owner.
pub(crate) struct ProjectFrontierPopulation<A> {
    pub(crate) id: PopulationId,
    pub(crate) source: Result<InputSource, OperatingError>,
    pub(crate) coverage: CoverageState,
    pub(crate) payload: ProjectFrontierPayload,
    pub(crate) admitted: OnceCell<Result<A, OperatingError>>,
}
#[derive(PartialEq, Eq)]
pub(crate) struct ProjectInvocationKey<R> {
    pub(crate) sources: Vec<PopulationId>,
    pub(crate) refused: Vec<ProjectRefusal>,
    pub(crate) policy: CompletionPolicy,
    pub(crate) bound: Vec<BoundKey<R>>,
}
#[derive(PartialEq, Eq)]
pub(crate) struct ProjectRefusal {
    pub(crate) source: Option<Result<InputSource, OperatingError>>,
    pub(crate) input: Input,
    pub(crate) error: OperatingError,
}
#[derive(PartialEq, Eq)]
pub(crate) enum BoundKey<R> {
    Current(PopulationId, CoverageState),
    Restored(R),
    Refused(OperatingError),
    Input(InputSource),
}

/// Structural equality is the only reuse decision accepted by the source interner.
pub(crate) enum ProjectSourceMatch {
    Equivalent,
    Distinct,
}
