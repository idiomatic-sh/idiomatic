//! The common owner preserves the existing graph retention/memo backend and its single budget.
use super::{
    construction::service as build,
    identity::service::{domain_identity, source},
};
use crate::application::file_dag::derived::ledger::lease::append_backing;
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use crate::application::file_dag::derived::owner::acquisition::population::types::{
    Acquisition, BackendRetention, InvocationStorage, OptionalRetention, StoredObservation,
};
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::graphs::types::handles::service::{
    DomainHandle, GraphHandle,
};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    GraphRequirement, GraphSelection, GraphViewIdentity, ViewContext,
};
use crate::application::file_dag::workspace::GraphInput;
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::GraphDomain,
    table_model::{
        service::{check_columns, kinds_of},
        types::Rows,
    },
};
use std::sync::Arc;
pub(in crate::application::file_dag::derived::owner) fn acquire(
    plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    input: &GraphInput<'_>,
    cache: &mut CacheBackend,
) -> Result<Option<GraphHandle>, OperatingError> {
    // Without a generation-stable admitted backing, use the existing ordinary
    // traversal rather than retain duplicate artifacts under fresh local IDs.
    if !input.canonical_backing() {
        return Ok(None);
    }
    let owner = input.owner();
    let population = input.population()?;
    let rows = input.rows();
    let admitted = owner.admit_graph(plan, input)?;

    let crate::domain::derived_views::types::ViewRequest::Adjacency { endpoints, keys } =
        &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    if cache.graph_enabled
        != crate::application::file_dag::derived::owner::cache::types::GraphAdmission::Enabled
    {
        return Ok(None);
    }
    let Some(source) = source(input, *endpoints, keys, &cache.eligible) else {
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
    let acquired = owner.try_acquire_shared_with_requirement(
        canonical,
        plan,
        &admitted,
        ViewContext::default(),
        StoredObservation::Unobserved,
        Some(&mut *cache),
        |cache| {
            invoked.set(true);
            let built = build::adjacency(owner, population, &admitted, plan, source, rows, cache)?;
            Ok(built.map(|built| Acquisition {
                storage: Arc::clone(&built.storage),
                construction: built.construction,
                backend: BackendRetention::Accounted,
                optional: OptionalRetention::Refused,
            }))
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
    let mut shared = cache
        .graphs
        .iter()
        .any(|entry| Arc::ptr_eq(&entry.graph, &graph));
    if !invoked.get() {
        cache.graph_counts.reuses.0 += 1;
        if !shared {
            shared = owner.retain_graph(
                cache,
                population,
                acquired.storage(),
                acquired.backings(),
                &graph,
            )? == GraphSharing::Shared;
        }
    }
    let sharing = if shared {
        GraphSharing::Shared
    } else {
        GraphSharing::Local
    };
    GraphHandle::new(owner, acquired, orientation, sharing).map(Some)
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
    population: &crate::domain::stored_graph::types::RelationPopulation,
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
    cache: &mut CacheBackend,
) -> Result<DomainHandle, OperatingError> {
    let owner = input.owner();
    let population = input.population()?;
    let rows = input.rows();
    access.acquired().view()?;
    let graph = access
        .acquired()
        .storage()
        .payload()
        .share_adjacency()
        .ok_or(OperatingError::WrongOperation)?;
    if !Arc::ptr_eq(&graph, access.raw_graph()) {
        return Err(OperatingError::NotRead);
    }
    let admitted = owner.admit_domain(plan, input, access.acquired())?;

    let crate::domain::derived_views::types::ViewRequest::Membership { column } = &plan.request
    else {
        return Err(OperatingError::WrongOperation);
    };
    validate_input(plan, rows)?;
    check_columns(kinds_of(rows), &[*column])?;
    let identity = input
        .canonical_backing()
        .then_some(&cache.eligible)
        .and_then(|eligible| domain_identity(input, *column, eligible));
    let physical_column = identity
        .as_ref()
        .map_or(*column, |identity| identity.column);
    let requirement = GraphViewIdentity::Dense {
        graph: graph_requirement(
            access
                .acquired()
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
    let acquired = if input.canonical_backing() {
        owner.acquire_shared_with_requirement(
            requirement,
            plan,
            &admitted,
            ViewContext::default(),
            StoredObservation::Unobserved,
            Some(&mut *cache),
            |cache| {
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
                Ok(Acquisition {
                    storage: built.storage,
                    construction: built.construction,
                    backend: BackendRetention::Accounted,
                    optional: OptionalRetention::Refused,
                })
            },
        )?
    } else {
        owner.acquire_invocation_with(plan, &admitted, access.backings(), || {
            invoked.set(true);
            let built = build::domain(
                owner,
                population,
                plan,
                &admitted,
                access,
                identity.clone(),
                rows,
                Some(&mut *cache),
            )?;
            Ok(InvocationStorage {
                storage: built.storage,
                construction: built.construction,
            })
        })?
    };
    let ryo_derived::domain::contract::types::DerivedView::Membership(
        ryo_derived::domain::contract::types::MembershipView::Dense(members),
    ) = acquired.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if members.len() != access.graph().vertices().len() {
        return Err(OperatingError::ShapeMismatch);
    }
    let members = acquired
        .storage()
        .payload()
        .share_marks()
        .ok_or(OperatingError::WrongOperation)?;
    let members = members.backing();
    let found = cache.graph_domains.iter().find(|entry| {
        Arc::ptr_eq(&entry.graph, access.raw_graph()) && Arc::ptr_eq(&entry.domain.members, members)
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
    for backing in access.backings().iter().copied() {
        append_backing(&mut backings, backing)?;
    }
    if let Some(rows) = domain
        .identity
        .as_ref()
        .and_then(|identity| identity.population.positions.as_ref())
    {
        append_backing(
            &mut backings,
            crate::application::file_dag::derived::ledger::types::Backing {
                key: BackingKey {
                    tag: BackingTag(1),
                    address: BackingAddress(Arc::as_ptr(rows) as usize),
                },
                bytes: crate::application::file_dag::derived::ledger::capacity::shared_vector(rows),
                retention_units: UnitCount(rows.capacity()),
            },
        )?;
    }
    append_backing(
        &mut backings,
        crate::application::file_dag::derived::ledger::types::Backing {
            key: BackingKey {
                tag: BackingTag(19),
                address: BackingAddress(Arc::as_ptr(&domain) as usize),
            },
            bytes: ByteCount(size_of::<GraphDomain>() + 2 * size_of::<usize>()),
            retention_units: UnitCount(0),
        },
    )?;
    let backings: Arc<[crate::application::file_dag::derived::ledger::types::Backing]> =
        backings.into();
    if !invoked.get() {
        owner.retain_domain(
            cache,
            population,
            access,
            &domain,
            acquired.storage(),
            &backings,
        )?;
    }
    DomainHandle::new(
        owner,
        access,
        acquired,
        domain,
        owner.required_lease(&backings)?,
    )
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
