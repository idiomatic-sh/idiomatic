//! Standalone application preparation uses the same required declarations as request preparation.
mod identity;
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};

/// All preparation routes delegate semantic metadata to the injected capability.
pub(in crate::application::file_dag) fn view(
    semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    request: &crate::domain::derived_views::types::ViewRequest,
    schemas: &[InputShape<'_>],
) -> Result<crate::domain::derived_views::types::contract::value::DerivedContract, OperatingError> {
    use crate::domain::derived_views::types::{contract::value::DerivedContract, ViewRequest};
    use ryo_derived::domain::request::types::DerivedRequest as SemanticRequest;
    let mut needed = crate::domain::table_cells::types::ColumnPlaces::default();
    let meaning = match request {
        ViewRequest::RawKeyOrder { stride } => SemanticRequest::RawKeyOrder {
            stride: ryo_derived::domain::raw_words::types::KeyStride(stride.0),
        },
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
    let access: crate::application::file_dag::derived::owner::types::DerivedAccess =
        semantic.into();
    let prepared = access.prepare(
        meaning,
        schemas.iter().map(|schema| schema.kinds.to_vec()).collect(),
    );
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
    let kind = crate::domain::derived_views::types::contract::value::kind_of(&request);
    kind.hash(&mut identity);
    let schemas = schemas
        .iter()
        .map(|schema| schema.kinds.to_vec())
        .collect::<Vec<_>>();
    schemas.hash(&mut identity);
    identity::hash_request(&request, &mut identity);
    Ok(DerivedContract::issue(
        crate::domain::derived_views::types::IdentityHash(identity.finish()),
        prepared,
        request,
        schemas,
        needed,
    ))
}
