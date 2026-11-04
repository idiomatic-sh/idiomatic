//! Generation-local sharing, exact identity and bounded admission through the real owner.

use crate::application::file_dag::derived::{indexes::LIMIT, owner::graph_fixture::GraphFixture};
use crate::domain::stored_graph::types::{GraphWalk, Orientation};
use crate::domain::table_model::types::{Rows, SelectedRows, SelectionIdentity, StoredSelection};
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;

const SOURCE: Input = Input::Relation(Relation::Function);
const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

fn rows(columns: &[FactColumn]) -> Rows<'_> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}

#[test]
fn shared_adjacency_builds_once_and_sparse_root_memo_avoids_retraversal() {
    let columns = [FactColumn(vec![1, 2, 1, 90]), FactColumn(vec![2, 3, 2, 91])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let first = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let mut walk = GraphWalk::default();
    let actual = fixture
        .reach(&first, &[], Value::Number(Integer(1)), &mut walk)
        .unwrap()
        .expect("memo admitted");
    let values: Vec<_> = actual
        .iter()
        .map(|vertex| first.graph.vertices()[*vertex as usize])
        .collect();
    assert_eq!(
        values,
        [Value::Number(Integer(2)), Value::Number(Integer(3))]
    );
    let first_state = fixture.snapshot();
    assert_eq!(
        (
            first_state.counts.builds,
            first_state.counts.traversals,
            first_state.counts.examined
        ),
        (1, 1, 3)
    );
    let second = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&first.graph, &second.graph));
    let again = fixture
        .reach(&second, &[], Value::Number(Integer(1)), &mut walk)
        .unwrap()
        .expect("memo hit");
    assert!(actual.same_data(&again));
    let shared = fixture.snapshot();
    assert_eq!(
        (
            shared.counts.builds,
            shared.counts.reuses,
            shared.counts.traversals,
            shared.counts.memo_hits
        ),
        (1, 1, 1, 1)
    );
    // Reversing endpoint columns shares adjacency but has a distinct root orientation.
    let reverse = fixture
        .acquire(SOURCE, &rows, [ENDPOINTS[1], ENDPOINTS[0]], &[])
        .unwrap()
        .unwrap();
    assert_eq!(reverse.orientation, Orientation::Reverse);
    assert!(Arc::ptr_eq(&first.graph, &reverse.graph));
    assert!(fixture.snapshot().bytes <= LIMIT);
}

#[test]
fn exact_selected_bindings_population_and_projection_determine_graph_identity() {
    let columns = [FactColumn(vec![1, 2, 90]), FactColumn(vec![2, 3, 91])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let selection = |bound, positions, columns| {
        Rows::Selected(SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: Relation::Function,
                identity: Arc::new(SelectionIdentity {
                    comparisons: vec![],
                    bindings: vec![crate::domain::table_model::types::ParameterBinding::Number(
                        ryo_values::domain::plan_parts::types::WholeNumber(bound),
                    )],
                }),
                rows: Arc::new(positions),
                columns,
                kinds: vec![CellKind::Number; 2],
            }),
        })
    };
    let first = selection(1, vec![0, 1], vec![ColumnIndex(0), ColumnIndex(1)]);
    let original = fixture
        .acquire(SOURCE, &first, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let reordered = selection(1, vec![0, 1], vec![ColumnIndex(1), ColumnIndex(0)]);
    let same = fixture
        .acquire(SOURCE, &reordered, [ENDPOINTS[1], ENDPOINTS[0]], &[])
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&original.graph, &same.graph));
    for different in [
        selection(2, vec![0, 1], vec![ColumnIndex(0), ColumnIndex(1)]),
        selection(1, vec![0, 2], vec![ColumnIndex(0), ColumnIndex(1)]),
    ] {
        let graph = fixture
            .acquire(SOURCE, &different, ENDPOINTS, &[])
            .unwrap()
            .unwrap();
        assert!(!Arc::ptr_eq(&original.graph, &graph.graph));
    }
    let other = fixture
        .acquire(
            Input::Relation(Relation::Identifier),
            &Rows::Stored(source),
            ENDPOINTS,
            &[],
        )
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&original.graph, &other.graph));
    assert_eq!(fixture.snapshot().counts.builds, 4);
}

#[test]
fn budget_refusal_keeps_complete_local_graph_without_retaining_handles() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function], &[], 0);
    let before = fixture.snapshot();
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(!graph.shared);
    let refused = fixture.snapshot();
    assert_eq!(refused.graphs, 0);
    assert_eq!(refused.optional_bytes, before.optional_bytes);
    assert_eq!(
        refused.graph_backings, 1,
        "required consumer keeps the local graph"
    );
    let mut walk = GraphWalk::default();
    assert!(
        fixture
            .reach(&graph, &[], Value::Number(Integer(1)), &mut walk)
            .unwrap()
            .is_none()
    );
    assert_eq!(walk.reached.len(), 2);
    assert_eq!(fixture.snapshot().counts.refusals, 1);
    drop(graph);
    assert_eq!(fixture.snapshot().graph_backings, 0);
    let absent = GraphFixture::new(&[], &[], LIMIT);
    assert!(
        absent
            .acquire(SOURCE, &rows, ENDPOINTS, &[])
            .unwrap()
            .is_none()
    );
    assert_eq!(absent.snapshot().graphs, 0);
}

#[test]
fn nearest_memo_is_shared_only_for_exact_domain_and_generation() {
    let columns = [FactColumn(vec![1, 2, 8]), FactColumn(vec![2, 3, 2])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])];
    let domain_rows = self::rows(&domain_columns);
    let membership = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let again = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&membership.domain, &again.domain));
    assert_eq!(
        fixture.snapshot().domains,
        1,
        "exact membership shares one domain"
    );
    let root = graph
        .graph
        .vertex(0, Value::Number(Integer(1)))
        .expect("stored vertex");
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    let first = fixture
        .nearest(
            &graph,
            &membership,
            root,
            Orientation::Forward,
            &mut local,
            &mut walk,
        )
        .unwrap();
    assert_eq!(
        first.map(|vertex| graph.graph.vertices()[vertex as usize]),
        Some(Value::Number(Integer(3)))
    );
    assert_eq!(fixture.snapshot().nearest, 1);
    assert_eq!(
        fixture
            .nearest(
                &graph,
                &again,
                root,
                Orientation::Forward,
                &mut local,
                &mut walk
            )
            .unwrap(),
        first
    );
    assert_eq!(fixture.snapshot().counts.forest_hits, 1);
    // A computed domain cannot borrow stable relation membership identity.
    let table = ryo_values::domain::rows::types::Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(2))],
        count: ryo_values::domain::rows::types::RowCount(1),
    };
    let different = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &Rows::Computed(&table),
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    local.clear();
    let changed = fixture
        .nearest(
            &graph,
            &different,
            root,
            Orientation::Forward,
            &mut local,
            &mut walk,
        )
        .unwrap();
    assert_eq!(
        changed.map(|vertex| graph.graph.vertices()[vertex as usize]),
        Some(Value::Number(Integer(2)))
    );
    assert_eq!(fixture.snapshot().nearest, 1);
    assert!(fixture.snapshot().memo_bytes <= super::service::MEMO_LIMIT);
    assert!(fixture.snapshot().bytes <= LIMIT);
}

#[test]
fn physical_partiality_ragged_views_and_next_generation_never_hit_old_graphs() {
    use crate::domain::coverage::types::{Lacking, LackingKind, Partial};
    use ryo_values::domain::stored_facts::types::PartialCause;
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let source = rows(&columns);
    let partials = [Partial {
        relation: Relation::Function,
        cause: PartialCause("missing kind".into()),
        lacking: Lacking::Kind(LackingKind("unknown".into())),
        at: None,
    }];
    let partial = GraphFixture::new(&[Relation::Function], &partials, LIMIT);
    assert!(
        partial
            .acquire(SOURCE, &source, ENDPOINTS, &[])
            .unwrap()
            .is_none()
    );
    let ragged = [FactColumn(vec![1, 2]), FactColumn(vec![2])];
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    assert!(
        fixture
            .acquire(SOURCE, &rows(&ragged), ENDPOINTS, &[])
            .unwrap()
            .is_none()
    );
    // The malformed physical population was a distinct file generation.
    fixture.clear();
    let old = fixture
        .acquire(SOURCE, &source, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    fixture.clear();
    let changed = [FactColumn(vec![1, 2]), FactColumn(vec![90, 91])];
    let new = fixture
        .acquire(SOURCE, &rows(&changed), ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&old.graph, &new.graph));
    assert_eq!(fixture.snapshot().counts.builds, 1);
    assert_eq!(fixture.snapshot().counts.reuses, 0);
    assert_ne!(old.graph.vertices(), new.graph.vertices());
}

#[test]
fn adjacency_admission_reclaims_dormant_payload_but_falls_back_while_the_index_is_live() {
    let columns = [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![2, 3, 4, 5])];
    let rows = rows(&columns);
    let probe = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let measured = probe
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let graph_cost = probe.graph_cost(&measured);
    assert!(graph_cost > 0);

    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let index = fixture
        .index(Relation::Function, &rows, &[ColumnIndex(0)])
        .unwrap();
    let before = fixture.snapshot();
    assert_eq!(before.indexes, 1);
    assert!(before.retained_view_bytes > 0);
    fixture.set_limit(before.bytes + graph_cost - before.retained_view_bytes);
    let local = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(!local.shared);
    let refused = fixture.snapshot();
    assert_eq!(refused.indexes, 1);
    assert_eq!(refused.counts.pinned_refusals, 1);
    assert_eq!(refused.graph_backings, 1);
    drop(index);
    let shared = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert!(shared.shared);
    let admitted = fixture.snapshot();
    assert_eq!(admitted.indexes, 0);
    assert_eq!(
        admitted.owner_entries_capacity,
        before.owner_entries_capacity
    );
    assert_eq!(admitted.counts.reclaimed_indexes, 1);
    assert_eq!(admitted.retained_view_bytes, 0);
    assert_eq!(
        admitted.counts.reclaimed_bytes,
        before.retained_view_bytes as u64
    );
    assert!(admitted.optional_bytes <= admitted.limit);
    assert!(Arc::ptr_eq(&local.graph, &shared.graph));
    assert_eq!(local.graph.vertices(), shared.graph.vertices());
    assert_eq!(local.graph.neighbors(), shared.graph.neighbors());
    assert_eq!(admitted.counts.builds, 1);
}

#[test]
fn nearest_handles_absent_parent_and_normal_parent_through_derived_forest() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![2]), FactColumn(vec![0])];
    let domain_rows = self::rows(&domain_columns);
    let membership = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    for (root, expected) in [(3, None), (1, Some(2))] {
        let root = graph.graph.vertex(0, Value::Number(Integer(root))).unwrap();
        let answer = fixture
            .nearest(
                &graph,
                &membership,
                root,
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .unwrap();
        assert_eq!(
            answer.map(|id| graph.graph.vertices()[id as usize]),
            expected.map(|n| Value::Number(Integer(n)))
        );
    }
}
