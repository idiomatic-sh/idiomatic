//! Physical graph kernels publish completed immutable storage to the common owner.
use super::types::{BuiltDomain, BuiltGraph};
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::ledger::lease::append_backing;
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use crate::application::file_dag::derived::owner::acquisition::admission::service::AdmittedViewInputs;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
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
    plan: &std::sync::Arc<crate::domain::derived_views::types::contract::value::DerivedContract>,
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
        cache.graph_counts.reuses.0 += 1;
        return Ok(Some(BuiltGraph {
            storage: Arc::clone(&entry.storage),
            construction: Construction::Reused,
        }));
    }
    let expected_identity = source.identity.clone();
    let storage = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::types::BuildRoute::GraphAdjacency.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
        || {
            owner.build_admitted(
                plan,
                admitted,
                crate::domain::derived_views::types::DerivedInputs::Adjacency {
                    rows: crate::domain::table_model::types::RepresentationInput::from(rows),
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
        .map(|shared| shared.0)
        .ok_or(OperatingError::WrongOperation)?;
    let storage = Arc::new(storage);
    let backings = owner.graph_backings(&storage)?;
    cache.graph_counts.builds.0 += 1;
    let shared =
        owner.retain_graph(cache, population, &storage, &backings, &graph)? == GraphSharing::Shared;
    tracing::trace!(target: constants::TARGET_GRAPH, event = constants::EVENT_ADJACENCY, reused = false, shared,
        relation = ?expected_identity.population.relation, vertices = graph.vertices().len(), edges = graph.neighbors(ryo_derived::domain::vertices::types::EdgeDirection::Forward).0.len(),
        graph_bytes = cache.graph_bytes.0, cache_bytes = cache.bytes.0, graph_enabled = ?cache.graph_enabled);
    Ok(Some(BuiltGraph {
        storage,
        construction: Construction::Built,
    }))
}

pub(in crate::application::file_dag::derived::owner::acquisition::graph) fn domain(
    owner: &DerivedOwner,
    population: PopulationId,
    plan: &Arc<crate::domain::derived_views::types::contract::value::DerivedContract>,
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
    if access.sharing() == GraphSharing::Shared
        && let (Some(cache), Some(identity)) = (cache.as_ref(), identity.as_ref())
        && let Some(entry) = cache.graph_domains.iter().find(|entry| {
            entry.population == population
                && Arc::ptr_eq(&entry.graph, access.raw_graph())
                && entry.domain.identity.as_ref() == Some(identity)
        })
    {
        return Ok(BuiltDomain {
            storage: Arc::clone(&entry.storage),
            construction: Construction::Reused,
        });
    }
    let storage = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::types::BuildRoute::GraphDomain.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
        || {
            owner.build_admitted(
                plan,
                admitted,
                crate::domain::derived_views::types::DerivedInputs::Membership {
                    rows: crate::domain::table_model::types::RepresentationInput::from(rows),
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
    if members.0.len() != access.graph().vertices().len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let domain = Arc::new(GraphDomain {
        identity,
        members,
    });
    let storage = Arc::new(storage);
    let mut backings = owner.graph_backings(&storage)?;
    let mut dependencies = backings.to_vec();
    for backing in access.backings().iter().copied() {
        append_backing(&mut dependencies, backing)?;
    }
    dependencies.push(
        crate::application::file_dag::derived::ledger::types::Backing {
            key: BackingKey {
                tag: BackingTag(19),
                address: BackingAddress(Arc::as_ptr(&domain) as usize),
            },
            bytes: ByteCount(size_of::<GraphDomain>() + 2 * size_of::<usize>()),
            retention_units: UnitCount(0),
        },
    );
    backings = dependencies.into();
    if let Some(cache) = cache.as_mut() {
        owner.retain_domain(cache, population, access, &domain, &storage, &backings)?;
    }
    tracing::trace!(target: constants::TARGET_GRAPH, event = constants::EVENT_DOMAIN, vertices = domain.members.0.len(), source_rows = count_of(rows).0, stable = domain.identity.is_some());
    Ok(BuiltDomain {
        storage,
        construction: Construction::Built,
    })
}
