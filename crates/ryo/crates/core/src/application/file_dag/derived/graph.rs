//! The common owner preserves the existing graph retention/memo backend and its single budget.
use super::graph_cache::service::{replace, room, slot};
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::derived::graph_cache::types::footprint::value::GraphFootprint;
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::{
        service::{domain_identity, source},
        types::{DomainEntry, GraphAccess, GraphDomain, GraphEntry},
    },
    table_model::{
        service::{check_columns, count_of, kinds_of},
        types::Rows,
    },
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;
pub(in crate::application::file_dag) fn acquire(
    plan: &std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
    admission: Option<&crate::application::file_dag::workspace::InvocationWorkspace>,
    input: Input,
    rows: &Rows<'_>,
    cache: Option<&mut CacheBackend>,
) -> Result<Option<GraphAccess>, OperatingError> {
    let crate::domain::derived_views::types::ViewRequest::Adjacency { endpoints, keys } =
        &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    let Some(cache) = cache.filter(|cache| cache.graph_enabled) else {
        return Ok(None);
    };
    let Some(source) = source(input, rows, *endpoints, keys, &cache.eligible) else {
        return Ok(None);
    };
    check_columns(kinds_of(rows), endpoints)?;
    check_columns(kinds_of(rows), keys)?;
    if let Some(entry) = cache
        .graphs
        .iter()
        .find(|entry| entry.graph.identity == source.identity)
    {
        cache.graph_counts.reuses += 1;
        return Ok(Some(GraphAccess {
            graph: (entry.graph).clone(),
            orientation: source.orientation,
            shared: true,
        }));
    }
    let orientation = source.orientation;
    let expected_identity = source.identity.clone();
    let mut storage = crate::domain::derived_views::types::DerivedStorage::Unbuilt;
    super::super::route_counts::observe(
        super::super::route_counts::BuildRoute::GraphAdjacency.label(),
        super::super::route_counts::derived(plan.kind),
        || {
            crate::domain::derived_views::collections::views::registration::registered(plan.kind)
                .build(
                    plan,
                    crate::domain::derived_views::types::DerivedInputs::Adjacency {
                        rows: crate::domain::operator::contract::views::BorrowedRows::from(rows),
                        source,
                    },
                    &mut storage,
                    &mut crate::domain::derived_views::types::DerivedScratch::default(),
                )
        },
    )?;
    let crate::domain::derived_views::types::DerivedView::Adjacency(graph) =
        crate::domain::derived_views::collections::views::registration::registered(plan.kind)
            .view(plan, &storage)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if graph.identity != expected_identity {
        return Err(OperatingError::ShapeMismatch);
    }
    let crate::domain::derived_views::types::DerivedStorage::Adjacency(graph) = storage else {
        return Err(OperatingError::WrongOperation);
    };
    cache.graph_counts.builds += 1;
    let graph = graph;
    let mut shared = false;
    if let (Some(cost), Some((replacement, metadata))) = (
        GraphFootprint::new(cache).graph(&graph),
        slot(&cache.graphs),
    ) {
        if let Some(cost) = cost.checked_add(metadata)
            && prepare_graph(cache, cost, admission)
            && let Some(bytes) = room(cache, cost)
        {
            replace(&mut cache.graphs, replacement);
            cache.graphs.push(GraphEntry {
                graph: (graph).clone(),
                roots: Vec::new(),
            });
            cache.graph_bytes += bytes - cache.bytes;
            cache.bytes = bytes;
            cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
            shared = true;
        }
    }
    if !shared {
        cache.graph_counts.refusals += 1;
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "adjacency", reused = false, shared,
        relation = ?graph.identity.population.relation, vertices = graph.vertices.len(), edges = graph.neighbors[0].len(),
        graph_bytes = cache.graph_bytes, cache_bytes = cache.bytes, graph_enabled = cache.graph_enabled);
    Ok(Some(GraphAccess {
        graph,
        orientation,
        shared,
    }))
}

pub(in crate::application::file_dag) fn domain(
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    access: &GraphAccess,
    input: Input,
    rows: &Rows<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<Arc<GraphDomain>, OperatingError> {
    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    let column = *column;
    check_columns(kinds_of(rows), &[column])?;
    let identity = cache
        .as_ref()
        .and_then(|cache| domain_identity(input, rows, column, &cache.eligible));
    if access.shared
        && let (Some(cache), Some(identity)) = (cache.as_ref(), identity.as_ref())
        && let Some(entry) = cache.graph_domains.iter().find(|entry| {
            entry.graph.identity == access.graph.identity
                && entry.domain.identity.as_ref() == Some(identity)
        })
    {
        return Ok((entry.domain).clone());
    }
    let mut storage = crate::domain::derived_views::types::DerivedStorage::Unbuilt;
    super::super::route_counts::observe(
        super::super::route_counts::BuildRoute::GraphDomain.label(),
        super::super::route_counts::derived(plan.kind),
        || {
            crate::domain::derived_views::collections::views::registration::registered(plan.kind)
                .build(
                    plan,
                    crate::domain::derived_views::types::DerivedInputs::Membership {
                        rows: crate::domain::operator::contract::views::BorrowedRows::from(rows),
                        space: crate::domain::derived_views::types::MembershipSpace::Dense(
                            &access.graph.vertices,
                        ),
                    },
                    &mut storage,
                    &mut crate::domain::derived_views::types::DerivedScratch::default(),
                )
        },
    )?;
    let crate::domain::derived_views::types::DerivedView::Membership(
        crate::domain::derived_views::types::MembershipData::Dense(members),
    ) = crate::domain::derived_views::collections::views::registration::registered(plan.kind)
        .view(plan, &storage)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if members.len() != access.graph.vertices.len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let crate::domain::derived_views::types::DerivedStorage::Membership(
        crate::domain::derived_views::types::MembershipData::Dense(members),
    ) = storage
    else {
        return Err(OperatingError::WrongOperation);
    };
    let domain = Arc::new(GraphDomain { identity, members });
    if access.shared
        && domain.identity.is_some()
        && let Some(cache) = cache.as_mut()
        && let (Some(cost), Some((replacement, metadata))) = (
            GraphFootprint::new(cache).domain(&domain),
            slot(&cache.graph_domains),
        )
        && let Some(bytes) = cost
            .checked_add(metadata)
            .and_then(|cost| room(cache, cost))
    {
        replace(&mut cache.graph_domains, replacement);
        cache.graph_domains.push(DomainEntry {
            graph: (access.graph).clone(),
            domain: (domain).clone(),
        });
        cache.graph_bytes += bytes - cache.bytes;
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "domain", vertices = domain.members.len(), source_rows = count_of(rows).0, stable = domain.identity.is_some());
    Ok(domain)
}

fn prepare_graph(
    cache: &mut CacheBackend,
    cost: usize,
    admission: Option<&crate::application::file_dag::workspace::InvocationWorkspace>,
) -> bool {
    if room(cache, cost).is_none() {
        if let Some(admission) = admission {
            admission.reclaim_derived(cache, cost);
        }
    }
    super::graph_cache::types::reclamation::value::GraphAdmission::new(cache).prepare(cost)
}
