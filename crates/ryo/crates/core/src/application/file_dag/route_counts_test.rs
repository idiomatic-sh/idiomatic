use super::*;
use ryo_derived::domain::contract::types::DerivedView;
fn capture(run: impl FnOnce()) -> State {
    STATE.with_borrow_mut(|state| {
        assert!(state.is_none());
        *state = Some(State {
            file: 7,
            origin: Origin::default(),
            counts: HashMap::new(),
            overflow: 0,
            rules: HashMap::new(),
        });
    });
    run();
    STATE.with_borrow_mut(Option::take).unwrap()
}
#[test]
fn physical_build_is_once_and_original_error_survives_nested_scopes() {
    let state = capture(|| {
        let _rule = rule(3, "example");
        let _node = node(19);
        let _owner = owner(7);
        event("owner_request", "Equality");
        let outcome: Result<(), &str> =
            observe("scoped_build", "Equality", || Err("original failure"));
        assert_eq!(outcome, Err("original failure"));
        event("owner_hit", "Equality");
    });
    assert_eq!(state.origin, Origin::default());
    let physical = state
        .counts
        .iter()
        .filter(|(key, _)| key.route.ends_with("build"))
        .collect::<Vec<_>>();
    assert_eq!(physical.len(), 1);
    assert_eq!(
        *physical[0].1,
        Count {
            calls: 1,
            successes: 0,
            failures: 1
        }
    );
    assert_eq!(
        physical[0].0.origin,
        Origin {
            rule: Some(3),
            node: Some(19),
            owner: true,
            owner_id: Some(7),
            ..Origin::default()
        }
    );
    assert_eq!(
        state.counts.values().map(|count| count.calls).sum::<u64>(),
        3
    );
}
#[test]
fn cross_rule_origin_is_attributed_and_disabled_recording_allocates_no_state() {
    let state = capture(|| {
        for number in [1, 2] {
            let _rule = rule(number, "example");
            let _node = node(41);
            observe("scoped_build", "PredicateMatches", || Ok::<_, ()>(())).unwrap();
        }
    });
    assert_eq!(state.counts.len(), 2);
    assert!(state.counts.keys().all(|key| key.origin.node == Some(41)));
    assert_eq!(
        state
            .counts
            .values()
            .map(|count| count.successes)
            .sum::<u64>(),
        2
    );
    observe("scoped_build", "Equality", || Ok::<_, ()>(())).unwrap();
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
    assert_eq!(state.overflow, 3);
}

#[test]
fn owner_identity_distinguishes_live_population_keys_and_rules_are_named_once() {
    let state = capture(|| {
        let _rule = rule(2, "query-name");
        let _node = node(41);
        let _owner = owner(17);
        for population in [3, 4] {
            let _identity = identity(29, [Some(population), None]);
            event("owner_request", "Membership");
        }
    });
    assert_eq!(state.rules.get(&2).map(String::as_str), Some("query-name"));
    assert_eq!(state.counts.len(), 2);
    assert!(
        state
            .counts
            .keys()
            .all(|key| key.origin.plan == Some(29) && key.origin.owner_id == Some(17))
    );
    let mut json = String::new();
    json_string(&mut json, "a\"b\n");
    assert_eq!(json, "\"a\\\"b\\n\"");
}

#[test]
fn owner_routes_count_each_physical_equality_build_once() {
    use crate::application::file_dag::derived::owner::{DerivedOwner, PopulationId, ViewContext};
    use crate::domain::{
        derived_views::types::*, operation_schema::types::InputSchema,
        operator::contract::views::BorrowedRows, operator::types::PreparedPool,
    };
    use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
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
            crate::outbound::derived::factory::test_port(),
            &ViewRequest::Equality {
                columns: on.to_vec(),
            },
            &[InputSchema {
                kinds: rows.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let state = capture(|| {
        let _rule = rule(2, "query-two");
        let _node = node(41);
        let owner = DerivedOwner::new(0, crate::outbound::derived::factory::test_port());
        owner.prepare_file(
            true,
            &[Relation::Function],
            &[],
            &[Relation::Function],
            &[],
            &PreparedPool::default(),
            |_| true,
        );
        let held = owner
            .acquire_stored(
                &plan,
                &[PopulationId(9)],
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                Some((Relation::Function, rows)),
            )
            .unwrap();
        assert!(matches!(held.view().unwrap(), DerivedView::Equality(_)));
        let selector_owner = DerivedOwner::new(0, crate::outbound::derived::factory::test_port());
        selector_owner.prepare_file(
            true,
            &[Relation::Function],
            &[],
            &[Relation::Function],
            &[],
            &PreparedPool::default(),
            |_| true,
        );
        let first = selector_owner
            .acquire_stored_with_route(
                BuildRoute::SelectorCandidate,
                &plan,
                &[PopulationId(10)],
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                Some((Relation::Function, rows)),
            )
            .unwrap();
        assert!(first.was_built());
        let second = selector_owner
            .acquire_stored_with_route(
                BuildRoute::SelectorCandidate,
                &plan,
                &[PopulationId(10)],
                ViewContext::default(),
                DerivedInputs::Rows(&[BorrowedRows::Stored(rows)]),
                &mut DerivedScratch::default(),
                Some((Relation::Function, rows)),
            )
            .unwrap();
        assert!(!second.was_built());
    });
    let builds = state
        .counts
        .iter()
        .filter(|(key, _)| key.route.ends_with("_build"))
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 2);
    for (route, owner) in [("owner_build", true), ("selector_candidate_build", true)] {
        let (key, count) = builds.iter().find(|(key, _)| key.route == route).unwrap();
        assert_eq!(key.origin.owner, owner);
        assert_eq!(count.calls, 1);
        assert_eq!(count.successes, 1);
        assert_eq!(count.failures, 0);
    }
}

use ryo_derived::domain::contract::types::DerivedKind;
