//! The recording profile and the workloads the route count table runs under it.
use super::super::constants::{KIND_EQUALITY, KIND_MEMBERSHIP, MAX_KEYS};
use super::super::types::{BuildRoute, RouteOrigin, RouteState, RouteTally};
use super::super::{service::event, service::identity, service::observe, service::owner};
use crate::application::file_dag::derived::constants::{OWNER_HIT, OWNER_REQUEST};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::derived::owner::service::ViewContext;
use crate::application::file_dag::derived::prepare::service::view;
use crate::application::file_dag::fixtures::program_derived_port;
use crate::application::file_dag::relations::service::RelationOwner;
use crate::application::file_dag::relations::types::ReuseEligibility;
use crate::application::file_dag::route_counts::profile::service::RouteProfile;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::derived_views::types::{
    DerivedInputs, DerivedScratch, ViewRequest,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::runs::types::RuleId;
use std::collections::HashMap;
use std::sync::Arc;

pub(super) const SCOPED_BUILD: &str = "scoped_build";
pub(super) const EXAMPLE_RULE: &str = "example";
pub(super) const QUERY_NAME: &str = "query-name";
pub(super) const QUERY_TWO: &str = "query-two";
pub(super) const ORIGINAL_FAILURE: &str = "original failure";
pub(super) fn recording() -> RouteState {
    RouteState {
        format: Arc::new(crate::outbound::route_counts::fixtures::fake::RouteCountsFake),
        file: 7.into(),
        origin: RouteOrigin::default(),
        counts: HashMap::new(),
        overflow: RouteTally::default(),
        rules: HashMap::new(),
        #[cfg(feature = "diagnostic-row-progress")]
        progress: super::service::new_progress(),
    }
}
/// Runs `run` with a recording profile handed to it, and answers what it recorded and what `run` answered.
pub(super) fn capture<R>(run: impl FnOnce(&RouteProfile) -> R) -> (RouteState, R) {
    let profile = RouteProfile(Some(Arc::new(std::sync::Mutex::new(recording()))));
    let answered = run(&profile);
    let state = profile.0.as_ref().expect("a recording profile");
    let mut state = state.lock().expect("an unpoisoned profile");
    (std::mem::replace(&mut *state, recording()), answered)
}
/// The profile calls a recording sees.
#[derive(Clone, Copy, Debug)]
pub(super) enum Workload {
    /// One rule, node and owner request, fail one scoped build and hit.
    FailedBuildInNestedScopes,
    /// The same scoped build under rules 1 and 2 at node 41, then once with recording disabled.
    BuildUnderTwoRules,
    /// One owner request at each of `MAX_KEYS + 3` nodes.
    RequestsPastTheKeyBound,
    /// Owner 17 requests plan 29 under populations 3 and 4.
    TwoLivePopulations,
    /// An owner acquires an equality view, and a second owner builds and then reuses it by the graph adjacency route.
    EqualityBuiltByTwoOwners,
}

/// What the workload's own calls answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// The observed build failed with this error.
    Failed(&'static str),
    /// A disabled profile holds recording state.
    Recording(bool),
    /// The acquired view is an equality view.
    Equality(bool),
    /// How an acquired view was constructed.
    Constructed(Construction),
}

/// Makes the workload's profile calls and answers what they answered.
pub(super) fn run_workload(
    profile: &RouteProfile,
    workload: Workload,
) -> Result<Vec<Answer>, String> {
    match workload {
        Workload::FailedBuildInNestedScopes => {
            let _rule = profile.rule(3, &RuleId(EXAMPLE_RULE.into()));
            let _node = profile.node(19);
            let _owner = owner(profile, 7);
            event(profile, OWNER_REQUEST, KIND_EQUALITY);
            let outcome: Result<(), &str> = observe(profile, SCOPED_BUILD, KIND_EQUALITY, || {
                Err(ORIGINAL_FAILURE)
            });
            event(profile, OWNER_HIT, KIND_EQUALITY);
            Ok(outcome.err().map(Answer::Failed).into_iter().collect())
        }
        Workload::BuildUnderTwoRules => {
            for number in [1, 2] {
                let _rule = profile.rule(number, &RuleId(EXAMPLE_RULE.into()));
                let _node = profile.node(41);
                observe(profile, SCOPED_BUILD, KIND_MEMBERSHIP, || Ok::<_, ()>(()))
                    .map_err(|()| "the scoped build failed".to_owned())?;
            }
            let disabled = RouteProfile::default();
            observe(&disabled, SCOPED_BUILD, KIND_EQUALITY, || Ok::<_, ()>(()))
                .map_err(|()| "the disabled build failed".to_owned())?;
            Ok(vec![Answer::Recording(disabled.0.is_some())])
        }
        Workload::RequestsPastTheKeyBound => {
            for number in 0..MAX_KEYS + 3 {
                let _node = profile.node(number);
                event(profile, OWNER_REQUEST, KIND_EQUALITY);
            }
            Ok(Vec::new())
        }
        Workload::TwoLivePopulations => {
            let _rule = profile.rule(2, &RuleId(QUERY_NAME.into()));
            let _node = profile.node(41);
            let _owner = owner(profile, 17);
            for population in [3, 4] {
                let _identity = identity(profile, 29, [Some(population), None]);
                event(profile, OWNER_REQUEST, KIND_MEMBERSHIP);
            }
            Ok(Vec::new())
        }
        Workload::EqualityBuiltByTwoOwners => {
            equality_built_by_two_owners(profile).map_err(|error| format!("{error:?}"))
        }
    }
}

/// An owner acquires an equality view of one stored column, then a second owner builds it by the
/// graph adjacency route and acquires it again.
pub(super) fn equality_built_by_two_owners(
    profile: &RouteProfile,
) -> Result<Vec<Answer>, OperatingError> {
    let columns = [FactColumn(vec![3, 1, 3])];
    let rows = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let plan = Arc::new(view(
        program_derived_port(),
        &ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[InputShape {
            kinds: rows.kinds,
            tree: None,
        }],
    )?);
    let _rule = profile.rule(2, &RuleId(QUERY_TWO.into()));
    let _node = profile.node(41);
    let recording = || {
        RelationOwner::recording(
            ReuseEligibility::Eligible,
            program_derived_port(),
            profile.clone(),
        )
    };
    let acquire = |owner: &RelationOwner, route: Option<BuildRoute>| {
        let proof = owner.admit_relation(Relation::Function, RepresentationInput::Stored(rows))?;
        let admitted = owner.derived().admit_view(&plan, owner, &[&proof])?;
        let inputs = [RepresentationInput::Stored(rows)];
        let inputs = DerivedInputs::Rows(&inputs);
        let scratch = &mut DerivedScratch::default();
        match route {
            Some(route) => owner.derived().acquire_stored_with_route(
                route,
                &plan,
                &admitted,
                ViewContext::default(),
                inputs,
                scratch,
                None,
            ),
            None => owner.derived().acquire_stored(
                &plan,
                &admitted,
                ViewContext::default(),
                inputs,
                scratch,
                None,
            ),
        }
    };
    let owner = recording();
    let held = acquire(&owner, None)?;
    let equality = matches!(held.view()?, DerivedView::Equality(_));
    let selector = recording();
    let first = acquire(&selector, Some(BuildRoute::GraphAdjacency))?;
    let second = acquire(&selector, Some(BuildRoute::GraphAdjacency))?;
    Ok(vec![
        Answer::Equality(equality),
        Answer::Constructed(first.construction()),
        Answer::Constructed(second.construction()),
    ])
}
