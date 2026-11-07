use super::constants::{
    KIND_EQUALITY, KIND_MEMBERSHIP, KIND_PREDICATE_MATCHES, MAX_KEYS, OWNER_BUILD,
    SELECTOR_CANDIDATE_BUILD,
};
use super::profile::service::STATE;
use super::schemas::document;
use super::types::{
    BuildRoute, RouteCount, RouteNodeIndex, RouteOrigin, RouteOwnerId, RouteOwnership, RoutePlanId,
    RouteRuleIndex, RouteRuleName, RouteState, RouteTally,
};
use super::{event, identity, node, observe, owner, rule};
use crate::application::file_dag::derived::constants::{OWNER_HIT, OWNER_REQUEST};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_values::domain::runs::types::RuleId;
use std::collections::HashMap;

const BUILD_SUFFIX: &str = "build";
const BUILD_ROUTE_SUFFIX: &str = "_build";
const SCOPED_BUILD: &str = "scoped_build";
const EXAMPLE_RULE: &str = "example";
const QUERY_NAME: &str = "query-name";
const QUERY_TWO: &str = "query-two";
const ORIGINAL_FAILURE: &str = "original failure";
const QUOTED_TEXT: &str = "a\"b\n";
const QUOTED_JSON: &str = "\"a\\\"b\\n\"";
fn capture(run: impl FnOnce()) -> RouteState {
    STATE.with_borrow_mut(|state| {
        assert!(state.is_none());
        *state = Some(RouteState {
            file: 7.into(),
            origin: RouteOrigin::default(),
            counts: HashMap::new(),
            overflow: RouteTally::default(),
            rules: HashMap::new(),
            #[cfg(feature = "diagnostic-row-progress")]
            progress: super::new_progress(),
        });
    });
    run();
    STATE.with_borrow_mut(Option::take).unwrap()
}
#[test]
fn physical_build_is_once_and_original_error_survives_nested_scopes() {
    let state = capture(|| {
        let _rule = rule(3, &RuleId(EXAMPLE_RULE.into()));
        let _node = node(19);
        let _owner = owner(7);
        event(OWNER_REQUEST, KIND_EQUALITY);
        let outcome: Result<(), &str> =
            observe(SCOPED_BUILD, KIND_EQUALITY, || Err(ORIGINAL_FAILURE));
        assert_eq!(outcome, Err(ORIGINAL_FAILURE));
        event(OWNER_HIT, KIND_EQUALITY);
    });
    assert_eq!(state.origin, RouteOrigin::default());
    let physical = state
        .counts
        .iter()
        .filter(|(key, _)| key.route.0.ends_with(BUILD_SUFFIX))
        .collect::<Vec<_>>();
    assert_eq!(physical.len(), 1);
    assert_eq!(
        *physical[0].1,
        RouteCount {
            calls: RouteTally(1),
            successes: RouteTally(0),
            failures: RouteTally(1)
        }
    );
    assert_eq!(
        physical[0].0.origin,
        RouteOrigin {
            #[cfg(feature = "diagnostic-row-progress")]
            namespace: super::types::RouteLabel(super::constants::DAG_NAMESPACE),
            rule: Some(RouteRuleIndex(3)),
            node: Some(RouteNodeIndex(19)),
            owner: RouteOwnership::Owned,
            owner_id: Some(RouteOwnerId(7)),
            ..RouteOrigin::default()
        }
    );
    assert_eq!(
        state
            .counts
            .values()
            .map(|count| count.calls.0)
            .sum::<u64>(),
        3
    );
}
#[test]
fn cross_rule_origin_is_attributed_and_disabled_recording_allocates_no_state() {
    let state = capture(|| {
        for number in [1, 2] {
            let _rule = rule(number, &RuleId(EXAMPLE_RULE.into()));
            let _node = node(41);
            observe(SCOPED_BUILD, KIND_PREDICATE_MATCHES, || Ok::<_, ()>(())).unwrap();
        }
    });
    assert_eq!(state.counts.len(), 2);
    assert!(
        state
            .counts
            .keys()
            .all(|key| key.origin.node == Some(RouteNodeIndex(41)))
    );
    assert_eq!(
        state
            .counts
            .values()
            .map(|count| count.successes.0)
            .sum::<u64>(),
        2
    );
    observe(SCOPED_BUILD, KIND_EQUALITY, || Ok::<_, ()>(())).unwrap();
    assert!(STATE.with_borrow(|state| state.is_none()));
}
#[test]
fn bounded_capture_reports_overflow_instead_of_claiming_complete_counts() {
    let state = capture(|| {
        for number in 0..MAX_KEYS + 3 {
            let _node = node(number);
            event("owner_request", "Equality");
        }
    });
    assert_eq!(state.counts.len(), MAX_KEYS);
    assert_eq!(state.overflow, RouteTally(3));
}

#[test]
fn owner_identity_distinguishes_live_population_keys_and_rules_are_named_once() {
    let state = capture(|| {
        let _rule = rule(2, &RuleId(QUERY_NAME.into()));
        let _node = node(41);
        let _owner = owner(17);
        for population in [3, 4] {
            let _identity = identity(29, [Some(population), None]);
            event(OWNER_REQUEST, KIND_MEMBERSHIP);
        }
    });
    assert_eq!(
        state.rules.get(&RouteRuleIndex(2)),
        Some(&RouteRuleName(QUERY_NAME.to_owned()))
    );
    assert_eq!(state.counts.len(), 2);
    assert!(
        state
            .counts
            .keys()
            .all(|key| key.origin.plan == Some(RoutePlanId(29))
                && key.origin.owner_id == Some(RouteOwnerId(17)))
    );
    assert_eq!(
        document(&RouteRuleName(QUOTED_TEXT.to_owned())).0,
        QUOTED_JSON
    );
}

#[test]
fn owner_routes_count_each_physical_equality_build_once() {
    use crate::application::file_dag::derived::owner::ViewContext;
    use crate::application::file_dag::relations::RelationOwner;
    use crate::domain::{
        derived_views::types::*, operation_schema::types::InputShape,
        operator::contract::views::BorrowedRows,
    };
    use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn};
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    use std::sync::Arc;
    let columns = [FactColumn(vec![3, 1, 3])];
    let rows = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let on = [ColumnIndex(0)];
    let plan = Arc::new(
        crate::application::file_dag::derived::prepare::view(
            crate::application::file_dag::fakes_test::program_derived_port(),
            &ViewRequest::Equality {
                columns: on.to_vec(),
            },
            &[InputShape {
                kinds: rows.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let state = capture(|| {
        let _rule = rule(2, &RuleId(QUERY_TWO.into()));
        let _node = node(41);
        let owner = RelationOwner::new(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            crate::application::file_dag::fakes_test::program_derived_port(),
        );
        let proof = owner
            .admit_relation(
                ryo_relations::domain::relations::types::Relation::Function,
                BorrowedRows::Stored(rows),
            )
            .unwrap();
        let admitted = owner.derived().admit_view(&plan, &owner, &[&proof]).unwrap();
        let held = owner
            .derived()
            .acquire_stored(
                &plan,
                &admitted,
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                None,
            )
            .unwrap();
        assert!(matches!(held.view().unwrap(), DerivedView::Equality(_)));
        let selector_owner = RelationOwner::new(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            crate::application::file_dag::fakes_test::program_derived_port(),
        );
        let proof = selector_owner
            .admit_relation(
                ryo_relations::domain::relations::types::Relation::Function,
                BorrowedRows::Stored(rows),
            )
            .unwrap();
        let admitted = selector_owner
            .derived()
            .admit_view(&plan, &selector_owner, &[&proof])
            .unwrap();
        let first = selector_owner
            .derived()
            .acquire_stored_with_route(
                BuildRoute::SelectorCandidate,
                &plan,
                &admitted,
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                None,
            )
            .unwrap();
        assert_eq!(first.construction(), Construction::Built);
        let second = selector_owner
            .derived()
            .acquire_stored_with_route(
                BuildRoute::SelectorCandidate,
                &plan,
                &admitted,
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                None,
            )
            .unwrap();
        assert_eq!(second.construction(), Construction::Reused);
    });
    let builds = state
        .counts
        .iter()
        .filter(|(key, _)| key.route.0.ends_with(BUILD_ROUTE_SUFFIX))
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 2);
    for route in [OWNER_BUILD, SELECTOR_CANDIDATE_BUILD] {
        let (key, count) = builds.iter().find(|(key, _)| key.route.0 == route).unwrap();
        assert_eq!(key.origin.owner, RouteOwnership::Owned);
        assert_eq!(count.calls, RouteTally(1));
        assert_eq!(count.successes, RouteTally(1));
        assert_eq!(count.failures, RouteTally(0));
    }
}

use ryo_derived::domain::contract::types::DerivedKind;
