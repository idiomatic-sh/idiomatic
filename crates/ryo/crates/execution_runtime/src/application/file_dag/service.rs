//! Prepare every operation once for one request, shared by all file workers: the domain prepares each operator and
//! the store's request views bind its derived views.
use ryo_execution_domain::domain::file_program::types::RequestOperators;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// Prepares every operation of the request once, against the catalog's relation schemas, shared by all file workers.
pub(crate) fn prepare(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
    semantic: impl Into<
        ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess,
    >,
) -> Result<RequestOperators, OperatingError> {
    prepare_with_schemas(plans, bindings, hosted, &kinds, semantic)
}
/// Prepares every operation of the request once, against the schemas `relation_kinds` answers.
pub(crate) fn prepare_with_schemas<'s>(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
    relation_kinds: &dyn Fn(Relation) -> &'s [CellKind],
    semantic: impl Into<
        ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess,
    >,
) -> Result<RequestOperators, OperatingError> {
    let mut views =
        ryo_execution_store::application::file_dag::derived::request::service::RequestViews::new(
            semantic,
        );
    ryo_execution_domain::domain::file_program::service::operators::prepare(
        plans,
        bindings,
        hosted,
        relation_kinds,
        &mut |contract| views.bind(contract),
    )
}
