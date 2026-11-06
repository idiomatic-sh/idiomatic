//! The common owner preserves the existing graph retention/memo backend and its single budget.
use super::{
    construction::service as build,
    identity::service::{domain_identity, source},
};
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::derived::graph_cache::types::handles::{
    DomainHandle, GraphAccess, GraphHandle,
};
use crate::application::file_dag::derived::owner::{
    DerivedOwner, GraphRequirement, GraphSelection, GraphViewIdentity, PopulationId,
    RequirementIdentity, ViewContext,
};
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
use std::sync::Arc;
pub(in crate::application::file_dag::derived::owner) fn acquire(
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
    let admitted = owner.admit_graph(plan, input, None)?;

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
    let acquired = owner.try_acquire_shared_with_requirement(
        canonical,
        plan,
        &admitted,
        ViewContext::default(),
        false,
        cache.as_deref_mut(),
        |cache| {
            invoked.set(true);
            let built = build::adjacency(owner, population, &admitted, plan, source, rows, cache)?;
            Ok(built.map(|built| (Arc::clone(&built.storage), built.built, true, false)))
        },
    )?;
    let Some(acquired) = acquired else {
        return Ok(None);
    };
    let ryo_derived::domain::contract::types::DerivedView::Adjacency(_) = acquired.view()? else {
        return Err(OperatingError::WrongOperation);
    };
    let graph = acquired
        .storage()
        .payload()
        .share_adjacency()
        .ok_or(OperatingError::WrongOperation)?;
    let mut shared = cache.as_ref().is_some_and(|cache| {
        cache
            .graphs
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.graph, &graph))
    });
    if !invoked.get()
        && let Some(cache) = cache.as_deref_mut()
    {
        cache.graph_counts.reuses += 1;
        if !shared {
            shared = owner.retain_graph(
                cache,
                population,
                acquired.storage(),
                acquired.backings(),
                &graph,
            )?;
        }
    }
    Ok(Some(GraphHandle {
        requirement,
        population,
        generation: input.generation(),
        built: acquired.was_built(),
        access: GraphAccess {
            graph: Arc::clone(&graph),
            orientation,
            shared,
        },
        backings: Arc::clone(acquired.backings()),
        acquired,
    }))
}
pub(super) fn graph_requirement(
    identity: &crate::domain::stored_graph::types::GraphIdentity,
) -> GraphRequirement {
    GraphRequirement {
        endpoints: identity.endpoints,
        keys: identity.keys.clone().into(),
        selection: graph_selection(&identity.population),
    }
}
pub(super) fn graph_selection(
    population: &crate::domain::stored_graph::types::Population,
) -> Option<GraphSelection> {
    population
        .positions
        .as_ref()
        .zip(population.selected.as_ref())
        .map(|(rows, identity)| GraphSelection::new(rows, identity))
}

pub(in crate::application::file_dag::derived::owner) fn domain(
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    access: &GraphHandle,
    input: &GraphInput<'_>,
    mut cache: Option<&mut CacheBackend>,
) -> Result<DomainHandle, OperatingError> {
    let owner = input.owner();
    let population = input.population()?;
    let rows = input.rows();
    access.acquired.view()?;
    let graph = access
        .acquired
        .storage()
        .payload()
        .share_adjacency()
        .ok_or(OperatingError::WrongOperation)?;
    if !Arc::ptr_eq(&graph, &access.graph) {
        return Err(OperatingError::NotRead);
    }
    let admitted = owner.admit_graph(plan, input, Some(&access.acquired))?;
    if !Arc::ptr_eq(&access.generation, &input.generation()) {
        return Err(OperatingError::NotRead);
    }

    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    validate_input(plan, rows)?;
    check_columns(kinds_of(rows), &[*column])?;
    let identity = cache
        .as_ref()
        .filter(|_| input.canonical_backing())
        .and_then(|cache| domain_identity(input, *column, &cache.eligible));
    let physical_column = identity
        .as_ref()
        .map_or(*column, |identity| identity.column);
    let requirement = GraphViewIdentity::Dense {
        graph: graph_requirement(
            access
                .acquired
                .storage()
                .graph_identity()
                .ok_or(OperatingError::WrongOperation)?
                .as_ref(),
        ),
        column: physical_column,
        selection: identity
            .as_ref()
            .and_then(|identity| graph_selection(&identity.population)),
    };
    let requirement = owner.canonical_graph_requirement(requirement, admitted.populations());
    let invoked = std::cell::Cell::new(false);
    let mut build = |cache| {
        invoked.set(true);
        let built = build::domain(
            owner,
            population,
            plan,
            &admitted,
            access,
            identity.clone(),
            rows,
            cache,
        )?;
        Ok((built.storage, built.built, true, false))
    };
    let acquired = if input.canonical_backing() {
        owner.acquire_shared_with_requirement(
            requirement,
            plan,
            &admitted,
            ViewContext::default(),
            false,
            cache.as_deref_mut(),
            build,
        )?
    } else {
        owner.acquire_invocation_with(plan, &admitted, &access.backings, || {
            build(cache.as_deref_mut()).map(|(storage, built, _, _)| (storage, built))
        })?
    };
    let ryo_derived::domain::contract::types::DerivedView::Membership(
        ryo_derived::domain::contract::types::MembershipView::Dense(members),
    ) = acquired.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if members.len() != access.graph.vertices().len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let members = acquired
        .storage()
        .payload()
        .share_marks()
        .ok_or(OperatingError::WrongOperation)?;
    let members = members.backing();
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
    let mut backings = acquired.backings().to_vec();
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
            crate::application::file_dag::derived::ledger::Backing {
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
        crate::application::file_dag::derived::ledger::Backing {
            key: (19, Arc::as_ptr(&domain) as usize),
            bytes: size_of::<GraphDomain>() + 2 * size_of::<usize>(),
            retention_units: 0,
        },
    )?;
    let backings: Arc<[crate::application::file_dag::derived::ledger::Backing]> = backings.into();
    if !invoked.get()
        && let Some(cache) = cache.as_deref_mut()
    {
        owner.retain_domain(
            cache,
            population,
            access,
            &domain,
            acquired.storage(),
            &backings,
        )?;
    }
    Ok(DomainHandle {
        _parent: access.acquired.clone(),
        _acquired: acquired.clone(),
        domain,
        built: acquired.was_built(),
        _lease: owner.required_lease(&backings)?,
    })
}

pub(super) fn append_backing(
    backings: &mut Vec<crate::application::file_dag::derived::ledger::Backing>,
    next: crate::application::file_dag::derived::ledger::Backing,
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

pub(super) fn validate_input(
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
