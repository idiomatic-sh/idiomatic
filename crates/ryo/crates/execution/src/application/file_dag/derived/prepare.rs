//! Standalone application preparation uses the same required declarations as request preparation.
use crate::domain::{
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    operator::{contract::types::OperatorContract, service::PreparationRoute},
    table_model::types::};
use ryo_values::domain::plans::types::Operation;

#[cfg(test)]
pub(crate) fn operator(
    item: PreparationRoute,
    operation: &Operation,
    schemas: &[InputShape<'_>],
    bindings: &[ParameterBinding],
) -> Result<OperatorContract, OperatingError> {
    let mut contract = item.prepare(operation, schemas, bindings)?;
    super::request::RequestViews::new(crate::outbound::derived::factory::test_port())
        .bind(&mut contract)?;
    Ok(contract)
}

/// All preparation routes delegate semantic metadata to the injected capability.
pub(in crate::application::file_dag) fn view(
    semantic: std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    request: &crate::domain::derived_views::types::ViewRequest,
    schemas: &[InputShape<'_>],
) -> Result<crate::domain::derived_views::types::DerivedContract, OperatingError> {
    use crate::domain::derived_views::types::{DerivedContract, ViewRequest};
    use ryo_derived::domain::contract::types::ViewRequest as SemanticRequest;
    let mut needed = crate::domain::table_cells::types::ColumnPlaces::default();
    let meaning = match request {
        ViewRequest::Equality { columns } => SemanticRequest::Equality {
            columns: columns.clone(),
        },
        ViewRequest::PredicateMatches(selector) => {
            crate::domain::text_gate::service::needed_of(
                &selector.identity.comparisons,
                &mut needed,
            );
            SemanticRequest::PredicateMatches {
                needed: needed
                    .0
                    .iter()
                    .map(|at| {
                        ryo_values::domain::plan_parts::types::ColumnIndex(
                            u32::try_from(*at).unwrap_or(u32::MAX),
                        )
                    })
                    .collect(),
            }
        }
        ViewRequest::CandidatePostings { column, literals } => SemanticRequest::CandidatePostings {
            column: *column,
            literals: literals.clone(),
        },
        ViewRequest::DistinctRepresentatives { columns } => {
            SemanticRequest::DistinctRepresentatives {
                columns: columns.clone(),
            }
        }
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
    let prepared = semantic.prepare(
        meaning,
        schemas.iter().map(|schema| schema.kinds.to_vec()).collect(),
    );
    let metadata = prepared
        .validate()
        .map_err(crate::domain::operator::contract::derived::portable_fault);
    // The engine's identity descriptor must describe the canonical literal slots produced by
    // semantic preparation, including duplicate elimination and ordering.
    let request = match (request, prepared.request()) {
        (
            ViewRequest::CandidatePostings { .. },
            SemanticRequest::CandidatePostings { column, literals },
        ) => ViewRequest::CandidatePostings {
            column: *column,
            literals: literals.clone(),
        },
        _ => request.clone(),
    };
    use std::hash::{Hash, Hasher};
    let mut identity = std::collections::hash_map::DefaultHasher::new();
    let kind = crate::domain::derived_views::service::kind(&request);
    kind.hash(&mut identity);
    let schemas = schemas
        .iter()
        .map(|schema| schema.kinds.to_vec())
        .collect::<Vec<_>>();
    schemas.hash(&mut identity);
    super::request::identity::hash_request(&request, &mut identity);
    Ok(DerivedContract {
        identity_hash: identity.finish(),
        kind: crate::domain::derived_views::service::kind(&request),
        request,
        schemas,
        metadata,
        needed,
        semantic: prepared,
    })
}

use ryo_values::domain::query_bindings::types::{ParameterBinding};
