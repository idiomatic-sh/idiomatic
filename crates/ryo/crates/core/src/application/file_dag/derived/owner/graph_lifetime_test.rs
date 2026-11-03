//! Real owner handles remain valid/accounted across clear and cannot poison refill.
use super::*;
use crate::domain::{
    stored_graph::types::{GraphWalk, Orientation},
    table_model::types::Rows,
};
use graph_fixture::GraphFixture;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::hash::BuildHasher;
fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
fn rows(columns: &[FactColumn]) -> Rows<'_> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}
const SOURCE: Input = Input::Relation(Relation::Function);
const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

#[test]
fn zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer() {
    let fixture = GraphFixture::new(&[Relation::Function], &[], 0);
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    let first = fixture
        .acquire(SOURCE, &input, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(!first.shared);
    let live = fixture.snapshot();
    assert_eq!(live.optional_bytes, 0);
    assert_eq!(live.graph_backings, 1);
    let reverse = fixture
        .acquire(SOURCE, &input, [ENDPOINTS[1], ENDPOINTS[0]], &[])
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&first.graph, &reverse.graph));
    assert_eq!(reverse.orientation, Orientation::Reverse);
    let shared = fixture.snapshot();
    assert_eq!(shared.unique_bytes, live.unique_bytes);
    assert_eq!(shared.graph_backings, 1);
    assert_eq!(shared.counts.builds, 1);
    let mut walk = GraphWalk::default();
    assert!(
        fixture
            .reach(&reverse, &[], number(3), &mut walk)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        walk.reached
            .iter()
            .map(|vertex| reverse.graph.vertices[*vertex as usize])
            .collect::<Vec<_>>(),
        [number(1), number(2)]
    );
    drop(first);
    assert_eq!(fixture.snapshot().graph_backings, 1);
    drop(reverse);
    let expired = fixture.snapshot();
    assert_eq!(expired.graph_backings, 0);
    assert_eq!(expired.optional_bytes, 0);
    let next = fixture
        .acquire(SOURCE, &input, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert_eq!(fixture.snapshot().counts.builds, 2);
    assert_eq!(fixture.snapshot().graph_backings, 1);
    drop(next);
    assert_eq!(fixture.snapshot().graph_backings, 0);
}

#[test]
fn old_completed_answer_owns_exact_context_through_clear_and_changed_facts() {
    let fixture = GraphFixture::new(
        &[Relation::Function, Relation::Identifier],
        &[],
        super::super::indexes::LIMIT,
    );
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    let graph = fixture
        .acquire(SOURCE, &input, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])];
    let domain_input = rows(&domain_columns);
    let domain = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_input,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let mut walk = GraphWalk::default();
    let root = graph.graph.vertex_ids[&(0, number(1))];
    let nearest = fixture
        .nearest(
            &graph,
            &domain,
            root,
            Orientation::Forward,
            &mut vec![],
            &mut walk,
        )
        .unwrap()
        .unwrap();
    assert_eq!(graph.graph.vertices[nearest as usize], number(3));
    let answer = fixture
        .reach(&graph, &[], number(1), &mut walk)
        .unwrap()
        .unwrap();
    let weak = answer.weak();
    assert_eq!(
        answer
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(2), number(3)]
    );
    let alias = fixture
        .reach(&graph, &[], number(1), &mut walk)
        .unwrap()
        .unwrap();
    assert!(answer.same_data(&alias));
    let live = fixture.snapshot();
    assert_eq!(live.graph_backings, 1);
    assert_eq!(
        live.unique_bytes,
        live.required_bytes + live.optional_bytes - live.overlap_bytes
    );
    drop(alias);
    fixture.clear();
    assert_eq!(fixture.snapshot().optional_bytes, 0);
    assert_eq!(fixture.snapshot().graph_backings, 1);
    let changed = [FactColumn(vec![1, 2]), FactColumn(vec![90, 91])];
    let next_input = rows(&changed);
    let next = fixture
        .acquire(SOURCE, &next_input, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let fresh = fixture
        .reach(&next, &[], number(1), &mut walk)
        .unwrap()
        .unwrap();
    assert_eq!(
        fresh
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(90)]
    );
    assert_eq!(fixture.snapshot().counts.builds, 1);
    assert_eq!(fixture.snapshot().graph_backings, 2);
    assert_eq!(
        answer
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(2), number(3)]
    );
    assert!(matches!(
        fixture.domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_input,
            ColumnIndex(0)
        ),
        Err(OperatingError::NotRead)
    ));
    drop(graph);
    drop(domain);
    drop(next);
    drop(fresh);
    fixture.clear();
    assert_eq!(
        fixture.snapshot().graph_backings,
        1,
        "the old completed answer alone keeps its exact dense space alive"
    );
    assert_eq!(
        answer
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(2), number(3)]
    );
    assert!(weak.upgrade().is_some());
    drop(answer);
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.snapshot().graph_backings, 0);
    assert_eq!(fixture.snapshot().unique_bytes, 0);
}

#[test]
fn weak_position_expiry_never_changes_map_key_hash_or_equality() {
    let positions = Arc::new(vec![0, 2]);
    let weak = Arc::downgrade(&positions);
    let descriptor = Arc::new(crate::domain::table_model::types::SelectionIdentity::default());
    let requirement = GraphRequirement {
        endpoints: ENDPOINTS,
        keys: Arc::from([]),
        selection: Some(GraphSelection::new(&positions, &descriptor)),
    };
    let key = Key {
        plan: RequirementIdentity::Graph(Arc::new(GraphViewIdentity::Adjacency(requirement))),
        populations: [PopulationId(7), PopulationId(usize::MAX)],
        len: 1,
        context: ViewContext::default(),
    };
    let mut map = hashbrown::HashMap::new();
    map.insert(key.clone(), 42);
    let hash = map.hasher().hash_one(&key);
    drop(positions);
    assert!(
        weak.upgrade().is_none(),
        "key metadata does not retain physical row positions"
    );
    assert_eq!(key, key.clone());
    assert_eq!(map.hasher().hash_one(&key), hash);
    assert_eq!(map.get(&key), Some(&42));
    let replacement = Arc::new(vec![0, 2]);
    let next =
        RequirementIdentity::Graph(Arc::new(GraphViewIdentity::Adjacency(GraphRequirement {
            endpoints: ENDPOINTS,
            keys: Arc::from([]),
            selection: Some(GraphSelection::new(&replacement, &descriptor)),
        })));
    assert_ne!(key.plan, next);
    assert!(
        !key.plan.compatible(&next),
        "expired metadata cannot authorize reuse for a new allocation"
    );
}

#[test]
fn unavailable_canonical_source_declines_without_retaining_duplicate_graphs() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    for (stable, max_views) in [(false, 1024), (true, 0)] {
        let fixture = GraphFixture::with_identity(
            &[Relation::Function],
            &[],
            super::super::indexes::LIMIT,
            stable,
            max_views,
        );
        for _ in 0..2 {
            assert!(
                fixture
                    .acquire(SOURCE, &input, ENDPOINTS, &[])
                    .unwrap()
                    .is_none(),
                "ordinary traversal handles sources without canonical identity"
            );
        }
        let observed = fixture.snapshot();
        assert_eq!(observed.graphs, 0);
        assert_eq!(observed.counts.builds, 0);
        assert_eq!(observed.optional_bytes, 0);
    }
}

#[test]
fn impossible_selected_position_admission_does_not_evict_dormant_index() {
    use crate::domain::table_model::types::{SelectedRows, SelectionIdentity, StoredSelection};
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    let mut fixture = GraphFixture::new(
        &[Relation::Function, Relation::Identifier],
        &[],
        super::super::indexes::LIMIT,
    );
    drop(
        fixture
            .index(Relation::Identifier, &input, &[ColumnIndex(0)])
            .unwrap(),
    );
    let before = fixture.snapshot();
    let mut positions =
        Vec::with_capacity(super::super::selection::MAX_RETAINED_POSITION_CAPACITY + 1);
    positions.extend([0, 1]);
    let Rows::Stored(source) = input else {
        unreachable!()
    };
    let selected = Rows::Selected(SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            identity: Arc::new(SelectionIdentity::default()),
            rows: Arc::new(positions),
            columns: ENDPOINTS.to_vec(),
            kinds: vec![CellKind::Number; 2],
        }),
    });
    let first = fixture
        .acquire(SOURCE, &selected, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(!first.shared);
    assert_eq!(
        first.graph.vertices,
        vec![1, 2, 3].into_iter().map(number).collect::<Vec<_>>()
    );
    let cost = fixture.graph_cost(&first);
    fixture.set_limit(before.bytes + cost - before.retained_view_bytes / 2);
    let second = fixture
        .acquire(SOURCE, &selected, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(
        !second.shared,
        "a repeated refusal cannot destructively reclaim unrelated cache entries"
    );
    assert!(Arc::ptr_eq(&first.graph, &second.graph));
    let after = fixture.snapshot();
    assert_eq!(
        (after.bytes, after.optional_bytes, after.indexes),
        (before.bytes, before.optional_bytes, before.indexes)
    );
    assert_eq!(after.counts.reclaimed_bytes, 0);
    assert_eq!(after.counts.builds, 1);
}

#[test]
fn exhausted_domain_identity_uses_local_membership_without_retained_duplicates() {
    let fixture = GraphFixture::with_identity(
        &[Relation::Function, Relation::Identifier],
        &[],
        super::super::indexes::LIMIT,
        true,
        1,
    );
    let edges = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let graph = fixture
        .acquire(SOURCE, &rows(&edges), ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(graph.shared);
    let domain_columns = [FactColumn(vec![2, 3]), FactColumn(vec![0, 0])];
    let input = rows(&domain_columns);
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    for _ in 0..2 {
        let domain = fixture
            .domain(
                &graph,
                Input::Relation(Relation::Identifier),
                &input,
                ColumnIndex(0),
            )
            .unwrap()
            .unwrap();
        assert!(
            domain.identity.is_none(),
            "an exhausted source publication cannot grant stable membership identity"
        );
        let nearest = fixture
            .nearest(
                &graph,
                &domain,
                graph.graph.vertex_ids[&(0, number(1))],
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .unwrap();
        assert_eq!(
            nearest.map(|at| graph.graph.vertices[at as usize]),
            Some(number(2))
        );
    }
    let observed = fixture.snapshot();
    assert_eq!(observed.graphs, 1);
    assert_eq!(observed.domains, 0);
    assert_eq!(observed.nearest, 0);
    assert_eq!(observed.counts.builds, 1);
}
