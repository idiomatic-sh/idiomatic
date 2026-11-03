//! Physical graph kernels publish completed immutable storage to the common owner.
use super::*;

pub(super) fn adjacency(
    owner: &DerivedOwner,
    population: PopulationId,
    generation: Arc<()>,
    plan: &std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
    source: crate::domain::stored_graph::types::GraphSource,
    rows: &Rows<'_>,
    cache: Option<&mut CacheBackend>,
) -> Result<Option<GraphHandle>, OperatingError> {
    let Some(cache) = cache else {
        return Err(OperatingError::NotRun);
    };
    let requirement = graph_requirement(&source.identity);
    if let Some(entry) = cache
        .graphs
        .iter()
        .find(|entry| entry.population == population && entry.graph.identity == source.identity)
    {
        cache.graph_counts.reuses += 1;
        return Ok(Some(GraphHandle {
            requirement,
            population,
            generation,
            built: false,
            access: GraphAccess {
                graph: Arc::clone(&entry.graph),
                orientation: source.orientation,
                shared: true,
            },
            _storage: Arc::clone(&entry.storage),
            backings: Arc::clone(&entry.backings),
            _lease: owner.required_lease(&entry.backings)?,
        }));
    }
    let orientation = source.orientation;
    let expected_identity = source.identity.clone();
    let mut storage = crate::domain::derived_views::types::DerivedStorage::Unbuilt;
    crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::GraphAdjacency.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
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
    let storage = Arc::new(storage);
    let crate::domain::derived_views::types::DerivedStorage::Adjacency(graph) = storage.as_ref()
    else {
        return Err(OperatingError::WrongOperation);
    };
    let graph = Arc::clone(graph);
    let backings = owner.graph_backings(&storage)?;
    let lease = owner.required_lease(&backings)?;
    cache.graph_counts.builds += 1;
    let shared = owner.retain_graph(cache, population, &storage, &backings, &graph)?;
    tracing::trace!(target: "ryo::diagnostic::graph", event = "adjacency", reused = false, shared,
        relation = ?graph.identity.population.relation, vertices = graph.vertices.len(), edges = graph.neighbors[0].len(),
        graph_bytes = cache.graph_bytes, cache_bytes = cache.bytes, graph_enabled = cache.graph_enabled);
    Ok(Some(GraphHandle {
        requirement,
        population,
        generation,
        built: true,
        access: GraphAccess {
            graph,
            orientation,
            shared,
        },
        _storage: storage,
        backings,
        _lease: lease,
    }))
}

pub(super) fn domain(
    owner: &DerivedOwner,
    population: PopulationId,
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    access: &GraphHandle,
    identity: Option<crate::domain::stored_graph::types::DomainIdentity>,
    rows: &Rows<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<DomainHandle, OperatingError> {
    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    let column = *column;
    check_columns(kinds_of(rows), &[column])?;
    if access.shared
        && let (Some(cache), Some(identity)) = (cache.as_ref(), identity.as_ref())
        && let Some(entry) = cache.graph_domains.iter().find(|entry| {
            entry.population == population
                && Arc::ptr_eq(&entry.graph, &access.graph)
                && entry.domain.identity.as_ref() == Some(identity)
        })
    {
        return Ok(DomainHandle {
            _graph: Arc::clone(&entry.graph_storage),
            built: false,
            domain: Arc::clone(&entry.domain),
            _storage: Arc::clone(&entry.storage),
            _lease: owner.required_lease(&entry.backings)?,
        });
    }
    let mut storage = crate::domain::derived_views::types::DerivedStorage::Unbuilt;
    crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::GraphDomain.label(),
        crate::application::file_dag::route_counts::derived(plan.kind),
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
    let storage = Arc::new(storage);
    let crate::domain::derived_views::types::DerivedStorage::Membership(
        crate::domain::derived_views::types::MembershipData::Dense(members),
    ) = storage.as_ref()
    else {
        return Err(OperatingError::WrongOperation);
    };
    let domain = Arc::new(GraphDomain {
        identity,
        members: Arc::clone(members),
    });
    let mut backings = owner.graph_backings(&storage)?;
    let mut dependencies = backings.to_vec();
    for backing in access.backings.iter().copied() {
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
    let lease = owner.required_lease(&backings)?;
    if let Some(cache) = cache.as_mut() {
        owner.retain_domain(cache, population, access, &domain, &storage, &backings)?;
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "domain", vertices = domain.members.len(), source_rows = count_of(rows).0, stable = domain.identity.is_some());
    Ok(DomainHandle {
        _graph: Arc::clone(&access._storage),
        built: true,
        domain,
        _storage: storage,
        _lease: lease,
    })
}
