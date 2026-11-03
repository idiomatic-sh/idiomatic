//! The common owner preserves the existing graph retention/memo backend and its single budget.
use super::graph_cache::types::handles::{DomainHandle, GraphAccess, GraphHandle};
use super::owner::{
    DerivedOwner, GraphRequirement, GraphSelection, GraphViewIdentity, PopulationId,
    RequirementIdentity, ViewContext,
};
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::workspace::GraphInput;
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::GraphDomain,
    table_model::{
        service::{check_columns, count_of, kinds_of},
        types::Rows,
    },
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
mod build;
mod identity;
use identity::{domain_identity, source};
use std::sync::Arc;
pub(in crate::application::file_dag::derived) fn acquire(
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    input: &GraphInput<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<Option<GraphHandle>, OperatingError> {
    // Without a generation-stable admitted backing, use the existing ordinary
    // traversal rather than retain duplicate artifacts under fresh local IDs.
    if !input.canonical_backing() {
        return Ok(None);
    }
    let owner = input.owner();
    let population = input.population()?;
    let rows = input.rows();

    let crate::domain::derived_views::types::ViewRequest::Adjacency { endpoints, keys } =
        &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    let Some(backend) = cache.as_ref().filter(|cache| cache.graph_enabled) else {
        return Ok(None);
    };
    let Some(source) = source(input, *endpoints, keys, &backend.eligible) else {
        return Ok(None);
    };
    validate_input(plan, rows)?;
    check_columns(kinds_of(rows), endpoints)?;
    check_columns(kinds_of(rows), keys)?;
    let requirement = graph_requirement(&source.identity);
    let orientation = source.orientation;
    let invoked = std::cell::Cell::new(false);
    let canonical =
        owner.canonical_graph_requirement(GraphViewIdentity::Adjacency(requirement), &[population]);
    let RequirementIdentity::Graph(view) = &canonical else {
        return Err(OperatingError::WrongOperation);
    };
    let GraphViewIdentity::Adjacency(requirement) = view.as_ref() else {
        return Err(OperatingError::WrongOperation);
    };
    let requirement = requirement.clone();
    let acquired = owner.acquire_shared_with_requirement(
        canonical,
        plan,
        &[population],
        ViewContext::default(),
        false,
        cache.as_deref_mut(),
        |cache| {
            invoked.set(true);
            let built = build::adjacency(
                owner,
                population,
                input.generation(),
                plan,
                source,
                rows,
                cache,
            )?
            .ok_or(OperatingError::NotRun)?;
            Ok((Arc::clone(&built._storage), built.built, true, false))
        },
    )?;
    let crate::domain::derived_views::types::DerivedView::Adjacency(_) = acquired.view()? else {
        return Err(OperatingError::WrongOperation);
    };
    let crate::domain::derived_views::types::DerivedStorage::Adjacency(graph) =
        acquired.storage.as_ref()
    else {
        return Err(OperatingError::WrongOperation);
    };
    let mut shared = cache.as_ref().is_some_and(|cache| {
        cache
            .graphs
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.graph, graph))
    });
    if !invoked.get()
        && let Some(cache) = cache.as_deref_mut()
    {
        cache.graph_counts.reuses += 1;
        if !shared {
            shared = owner.retain_graph(
                cache,
                population,
                &acquired.storage,
                &acquired.backings,
                graph,
            )?;
        }
    }
    Ok(Some(GraphHandle {
        requirement,
        population,
        generation: input.generation(),
        built: acquired.built,
        access: GraphAccess {
            graph: Arc::clone(graph),
            orientation,
            shared,
        },
        _storage: acquired.storage,
        backings: acquired.backings,
        _lease: acquired.lease,
    }))
}
fn graph_requirement(
    identity: &crate::domain::stored_graph::types::GraphIdentity,
) -> GraphRequirement {
    GraphRequirement {
        endpoints: identity.endpoints,
        keys: identity.keys.clone().into(),
        selection: graph_selection(&identity.population),
    }
}
fn graph_selection(
    population: &crate::domain::stored_graph::types::Population,
) -> Option<GraphSelection> {
    population
        .positions
        .as_ref()
        .zip(population.selected.as_ref())
        .map(|(rows, identity)| GraphSelection::new(rows, identity))
}

pub(in crate::application::file_dag::derived) fn domain(
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    access: &GraphHandle,
    input: &GraphInput<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<DomainHandle, OperatingError> {
    let owner = input.owner();
    let population = input.population()?;
    let rows = input.rows();
    if !Arc::ptr_eq(&access.generation, &input.generation()) {
        return Err(OperatingError::NotRead);
    }

    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    validate_input(plan, rows)?;
    check_columns(kinds_of(rows), &[*column])?;
    if !input.canonical_backing() {
        // A readable local domain remains useful to the existing traversal,
        // but cannot authorize retained membership or nearest-answer identity.
        return build::domain(owner, population, plan, access, None, rows, cache);
    }
    let identity = cache
        .as_ref()
        .and_then(|cache| domain_identity(input, *column, &cache.eligible));
    let physical_column = identity
        .as_ref()
        .map_or(*column, |identity| identity.column);
    let requirement = GraphViewIdentity::Dense {
        graph: access.requirement.clone(),
        column: physical_column,
        selection: identity
            .as_ref()
            .and_then(|identity| graph_selection(&identity.population)),
    };
    let populations = [access.population, population];
    let populations = if populations[0] == populations[1] {
        &populations[..1]
    } else {
        &populations[..]
    };
    let requirement = owner.canonical_graph_requirement(requirement, populations);
    let invoked = std::cell::Cell::new(false);
    let acquired = owner.acquire_shared_with_requirement(
        requirement,
        plan,
        populations,
        ViewContext::default(),
        false,
        cache.as_deref_mut(),
        |cache| {
            invoked.set(true);
            let built = build::domain(
                owner,
                population,
                plan,
                access,
                identity.clone(),
                rows,
                cache,
            )?;
            Ok((Arc::clone(&built._storage), built.built, true, false))
        },
    )?;
    let crate::domain::derived_views::types::DerivedView::Membership(
        crate::domain::derived_views::types::MembershipData::Dense(members),
    ) = acquired.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if members.len() != access.graph.vertices.len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let crate::domain::derived_views::types::DerivedStorage::Membership(
        crate::domain::derived_views::types::MembershipData::Dense(members),
    ) = acquired.storage.as_ref()
    else {
        return Err(OperatingError::WrongOperation);
    };
    let found = cache.as_ref().and_then(|cache| {
        cache.graph_domains.iter().find(|entry| {
            Arc::ptr_eq(&entry.graph, &access.graph) && Arc::ptr_eq(&entry.domain.members, members)
        })
    });
    let domain = found.map_or_else(
        || {
            Arc::new(GraphDomain {
                identity,
                members: Arc::clone(members),
            })
        },
        |entry| Arc::clone(&entry.domain),
    );
    let mut backings = acquired.backings.to_vec();
    for backing in access.backings.iter().copied() {
        append_backing(&mut backings, backing)?;
    }
    if let Some(rows) = domain
        .identity
        .as_ref()
        .and_then(|identity| identity.population.positions.as_ref())
    {
        append_backing(
            &mut backings,
            super::ledger::Backing {
                key: (1, Arc::as_ptr(rows) as usize),
                bytes: size_of::<Vec<u32>>()
                    + 2 * size_of::<usize>()
                    + rows.capacity() * size_of::<u32>(),
                retention_units: rows.capacity(),
            },
        )?;
    }
    append_backing(
        &mut backings,
        super::ledger::Backing {
            key: (19, Arc::as_ptr(&domain) as usize),
            bytes: size_of::<GraphDomain>() + 2 * size_of::<usize>(),
            retention_units: 0,
        },
    )?;
    let backings: Arc<[super::ledger::Backing]> = backings.into();
    if !invoked.get()
        && let Some(cache) = cache.as_deref_mut()
    {
        owner.retain_domain(
            cache,
            population,
            access,
            &domain,
            &acquired.storage,
            &backings,
        )?;
    }
    Ok(DomainHandle {
        _graph: Arc::clone(&access._storage),
        domain,
        _storage: acquired.storage,
        built: acquired.built,
        _lease: owner.required_lease(&backings)?,
    })
}

fn append_backing(
    backings: &mut Vec<super::ledger::Backing>,
    next: super::ledger::Backing,
) -> Result<(), OperatingError> {
    if let Some(old) = backings.iter().find(|old| old.key == next.key) {
        if old.bytes != next.bytes || old.retention_units != next.retention_units {
            return Err(OperatingError::ShapeMismatch);
        }
    } else {
        backings.push(next);
    }
    Ok(())
}

fn validate_input(
    plan: &crate::domain::derived_views::types::DerivedContract,
    rows: &Rows<'_>,
) -> Result<(), OperatingError> {
    plan.metadata.as_ref().map_err(Clone::clone)?;
    if plan
        .schemas
        .first()
        .is_none_or(|schema| schema.as_slice() != kinds_of(rows))
    {
        return Err(OperatingError::ShapeMismatch);
    }
    Ok(())
}
