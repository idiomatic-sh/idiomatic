//! The `prepare` application operation.

use ryo_execution_domain::domain::{
    operating::errors::OperatingError, operation_schema::types::InputShape,
};

/// All preparation routes delegate semantic metadata to the injected capability.
///
/// # Errors
///
/// None in practice: a schema the semantic capability refuses is recorded in the contract, which its validation
/// returns.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::prepare::service::view;
/// ```
pub fn view(
    semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
    request: &ryo_execution_domain::domain::derived_views::types::ViewRequest,
    schemas: &[InputShape<'_>],
) -> Result<
    ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract,
    OperatingError,
> {
    use ryo_derived::domain::request::types::DerivedRequest as SemanticRequest;
    use ryo_execution_domain::domain::derived_views::types::{
        ViewRequest, contract::value::DerivedContract,
    };
    let meaning = match request {
        ViewRequest::Equality { columns } => SemanticRequest::Equality {
            columns: columns.clone(),
        },
        ViewRequest::Adjacency { endpoints, keys } => SemanticRequest::Adjacency {
            endpoints: *endpoints,
            keys: keys.clone(),
        },
        ViewRequest::Membership { column } => SemanticRequest::Membership { column: *column },
        ViewRequest::PartitionOrder { keys } => {
            SemanticRequest::PartitionOrder { keys: keys.clone() }
        }
        ViewRequest::ClosureAdjacency {
            source,
            destination,
        } => SemanticRequest::ClosureAdjacency {
            source: *source,
            destination: *destination,
        },
        ViewRequest::CycleAdjacency {
            source,
            destination,
        } => SemanticRequest::CycleAdjacency {
            source: *source,
            destination: *destination,
        },
        ViewRequest::OrderedKeys {
            left_on,
            right_on,
            left_order,
            right_order,
        } => SemanticRequest::OrderedKeys {
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            left_order: *left_order,
            right_order: *right_order,
        },
        ViewRequest::OrderedDfs(program) => SemanticRequest::OrderedDfs {
            root: program.root,
            root_keys: program.root_keys.clone(),
            source: program.source,
            destination: program.destination,
            order: program.order,
            edge: program.edge,
            edge_keys: program.edge_keys.clone(),
        },
        ViewRequest::NativeLookup {
            left_on,
            right_on,
            view,
        } => SemanticRequest::NativeLookup {
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            view: view.clone(),
        },
    };
    let access: crate::application::file_dag::derived::owner::service::DerivedAccess =
        semantic.into();
    let prepared = access.prepare(
        meaning,
        schemas.iter().map(|schema| schema.kinds.to_vec()).collect(),
    );
    let request = request.clone();
    use std::hash::{Hash, Hasher};
    let mut identity = std::collections::hash_map::DefaultHasher::new();
    let kind =
        ryo_execution_domain::domain::derived_views::types::contract::value::kind_of(&request);
    kind.hash(&mut identity);
    let schemas = schemas
        .iter()
        .map(|schema| schema.kinds.to_vec())
        .collect::<Vec<_>>();
    schemas.hash(&mut identity);
    super::identity::service::hash_request(&request, &mut identity);
    Ok(DerivedContract::issue(
        ryo_execution_domain::domain::derived_views::types::IdentityHash(identity.finish()),
        prepared,
        request,
        schemas,
    ))
}
