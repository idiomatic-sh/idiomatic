//! Physical graph kernels publish completed immutable storage to the common owner.
use super::types::{BuiltDomain, BuiltGraph};
use crate::application::file_dag::derived::ledger::append_backing;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AdmittedViewInputs;
use crate::application::file_dag::derived::{
    owner::DerivedOwner, owner::cache::types::CacheBackend,
    owner::graphs::types::handles::service::GraphHandle, population::types::PopulationId,
};
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::GraphDomain,
    table_model::{
        service::{check_columns, count_of, kinds_of},
        types::Rows,
    },
};
use std::sync::Arc;

pub(in crate::application::file_dag::derived::owner::acquisition::graph) fn adjacency(
    owner: &DerivedOwner,
    population: PopulationId,
    admitted: &AdmittedViewInputs<'_>,
    plan: &std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
    source: crate::domain::stored_graph::types::GraphSource,
    rows: &Rows<'_>,
    cache: Option<&mut CacheBackend>,
) -> Result<Option<BuiltGraph>, OperatingError> {
    let Some(cache) = cache else {
        return Err(OperatingError::NotRun);
    };
    if let Some(entry) = cache.graphs.iter().find(|entry| {
        entry.population == population
            && entry
                .storage
                .graph_identity()
                .is_some_and(|identity| identity.as_ref() == &source.identity)
    }) {
        cache.graph_counts.reuses += 1;
        return Ok(Some(BuiltGraph {
            storage: Arc::clone(&entry.storage),
            built: false,
        }));
    }
    let expected_identity = source.identity.clone();
    let storage = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::GraphAdjacency.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
        || {
            owner.build_admitted(
                plan,
                admitted,
                crate::domain::derived_views::types::DerivedInputs::Adjacency {
                    rows: crate::domain::operator::contract::views::BorrowedRows::from(rows),
                    source,
                },
                &mut crate::domain::derived_views::types::DerivedScratch::default(),
                None,
            )
        },
    )?;
    let Some(storage) = storage else {
        return Ok(None);
    };
    if storage.graph_identity().map(Arc::as_ref) != Some(&expected_identity) {
        return Err(OperatingError::ShapeMismatch);
    }
    let graph = storage
        .payload()
        .share_adjacency()
        .ok_or(OperatingError::WrongOperation)?;
    let storage = Arc::new(storage);
    let backings = owner.graph_backings(&storage)?;
    cache.graph_counts.builds += 1;
    let shared = owner.retain_graph(cache, population, &storage, &backings, &graph)?;
    tracing::trace!(target: "ryo::diagnostic::graph", event = "adjacency", reused = false, shared,
        relation = ?expected_identity.population.relation, vertices = graph.vertices().len(), edges = graph.neighbors()[0].len(),
        graph_bytes = cache.graph_bytes, cache_bytes = cache.bytes, graph_enabled = cache.graph_enabled);
    Ok(Some(BuiltGraph {
        storage,
        built: true,
    }))
}

pub(in crate::application::file_dag::derived::owner::acquisition::graph) fn domain(
    owner: &DerivedOwner,
    population: PopulationId,
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    admitted: &AdmittedViewInputs<'_>,
    access: &GraphHandle,
    identity: Option<crate::domain::stored_graph::types::DomainIdentity>,
    rows: &Rows<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<BuiltDomain, OperatingError> {
    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    let column = *column;
    check_columns(kinds_of(rows), &[column])?;
    if access.shared()
        && let (Some(cache), Some(identity)) = (cache.as_ref(), identity.as_ref())
        && let Some(entry) = cache.graph_domains.iter().find(|entry| {
            entry.population == population
                && Arc::ptr_eq(&entry.graph, access.raw_graph())
                && entry.domain.identity.as_ref() == Some(identity)
        })
    {
        return Ok(BuiltDomain {
            storage: Arc::clone(&entry.storage),
            built: false,
        });
    }
    let storage = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::GraphDomain.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
        || {
            owner.build_admitted(
                plan,
                admitted,
                crate::domain::derived_views::types::DerivedInputs::Membership {
                    rows: crate::domain::operator::contract::views::BorrowedRows::from(rows),
                    space: crate::domain::derived_views::types::MembershipSpace::Dense(
                        access.graph().vertices(),
                    ),
                },
                &mut crate::domain::derived_views::types::DerivedScratch::default(),
                None,
            )
        },
    )?
    .ok_or(OperatingError::WrongOperation)?;
    let members = storage
        .payload()
        .share_marks()
        .ok_or(OperatingError::WrongOperation)?;
    if members.as_slice().len() != access.graph().vertices().len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let domain = Arc::new(GraphDomain {
        identity,
        members: Arc::clone(members.backing()),
    });
    let storage = Arc::new(storage);
    let mut backings = owner.graph_backings(&storage)?;
    let mut dependencies = backings.to_vec();
    for backing in access.backings().iter().copied() {
        append_backing(&mut dependencies, backing)?;
    }
    if let Some(rows) = domain
        .identity
        .as_ref()
        .and_then(|id| id.population.positions.as_ref())
    {
        append_backing(
            &mut dependencies,
            crate::application::file_dag::derived::ledger::Backing {
                key: (1, Arc::as_ptr(rows) as usize),
                bytes: size_of::<Vec<u32>>()
                    + 2 * size_of::<usize>()
                    + rows.capacity() * size_of::<u32>(),
                retention_units: rows.capacity(),
            },
        )?;
    }
    dependencies.push(crate::application::file_dag::derived::ledger::Backing {
        key: (19, Arc::as_ptr(&domain) as usize),
        bytes: size_of::<GraphDomain>() + 2 * size_of::<usize>(),
        retention_units: 0,
    });
    backings = dependencies.into();
    if let Some(cache) = cache.as_mut() {
        owner.retain_domain(cache, population, access, &domain, &storage, &backings)?;
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "domain", vertices = domain.members.len(), source_rows = count_of(rows).0, stable = domain.identity.is_some());
    Ok(BuiltDomain {
        storage,
        built: true,
    })
}
