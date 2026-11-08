//! Generation-local sharing, exact identity and bounded admission through the real owner.

use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;

pub(super) const SOURCE: Input = Input::Relation(Relation::Function);
pub(super) const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

pub(super) fn rows(columns: &[FactColumn]) -> Rows<'_> {
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
        .map(|vertex| first.graph().vertices()[*vertex as usize])
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
    assert!(Arc::ptr_eq(first.raw_graph(), second.raw_graph()));
    let again = fixture
        .reach(&second, &[], Value::Number(Integer(1)), &mut walk)
        .unwrap()
        .expect("memo hit");
    assert_eq!(actual.data_identity(), again.data_identity());
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
    assert_eq!(reverse.orientation(), Orientation::Reverse);
    assert!(Arc::ptr_eq(first.raw_graph(), reverse.raw_graph()));
    assert!(fixture.snapshot().bytes <= LIMIT);
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
    assert_eq!(graph.sharing(), GraphSharing::Local);
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
fn physical_partiality_ragged_views_and_next_generation_never_hit_old_graphs() {
    use ryo_execution_domain::domain::coverage::types::{Lacking, LackingKind, Partial};
    use ryo_values::domain::stored_facts::types::PartialCause;
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let source = rows(&columns);
    let partials = [Partial {
        relation: Relation::Function,
        cause: PartialCause("missing kind".into()),
        lacking: Lacking::Kind(LackingKind("unknown".into())),
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
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
    assert!(!Arc::ptr_eq(old.raw_graph(), new.raw_graph()));
    assert_eq!(fixture.snapshot().counts.builds, 1);
    assert_eq!(fixture.snapshot().counts.reuses, 0);
    assert_ne!(old.graph().vertices(), new.graph().vertices());
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
    assert_eq!(local.sharing(), GraphSharing::Local);
    let refused = fixture.snapshot();
    assert_eq!(refused.indexes, 1);
    assert_eq!(refused.counts.pinned_refusals, 1);
    assert_eq!(refused.graph_backings, 1);
    drop(index);
    let shared = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert_eq!(shared.sharing(), GraphSharing::Shared);
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
    assert!(Arc::ptr_eq(local.raw_graph(), shared.raw_graph()));
    assert_eq!(local.graph().vertices(), shared.graph().vertices());
    assert_eq!(
        local.graph().neighbors(ryo_derived::domain::vertices::types::EdgeDirection::Forward),
        shared.graph().neighbors(ryo_derived::domain::vertices::types::EdgeDirection::Forward)
    );
    assert_eq!(
        local.graph().neighbors(ryo_derived::domain::vertices::types::EdgeDirection::Backward),
        shared.graph().neighbors(ryo_derived::domain::vertices::types::EdgeDirection::Backward)
    );
    assert_eq!(admitted.counts.builds, 1);
}

#[test]
fn retired_immutable_handles_keep_their_lease_but_cannot_authorize_new_demands() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![3]), FactColumn(vec![0])];
    let domain = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &self::rows(&domain_columns),
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let before = fixture.snapshot();
    assert_eq!(before.graphs, 1);
    assert_eq!(before.domains, 1);
    assert!(
        before.required_bytes > 0,
        "actual graph and domain retain required leases"
    );
    let vertices = graph.graph().vertices().to_vec();
    let members = domain.members().0.to_vec();
    fixture.clear();
    let retired = fixture.snapshot();
    assert_eq!(retired.graphs, 0);
    assert_eq!(retired.domains, 0);
    assert_eq!(retired.retained_view_bytes, 0);
    assert_eq!(retired.optional_bytes, 0);
    assert!(retired.required_bytes > 0, "held payloads remain charged");
    assert!(
        retired.required_bytes < before.required_bytes,
        "retiring cached publications releases their required metadata"
    );
    assert_eq!(
        retired.unique_bytes, retired.required_bytes,
        "only held required backings remain after cache retirement"
    );
    assert_eq!(
        graph.graph().vertices(),
        vertices,
        "held immutable graph remains leased"
    );
    assert_eq!(
        domain.members().0,
        members,
        "held immutable domain remains leased"
    );
    assert!(matches!(
        fixture.reach(
            &graph,
            &[],
            Value::Number(Integer(1)),
            &mut GraphWalk::default()
        ),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
    ));
    assert!(matches!(
        fixture.nearest(
            &graph,
            &domain,
            0,
            Orientation::Forward,
            &mut vec![],
            &mut GraphWalk::default()
        ),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
    ));
    drop(domain);
    drop(graph);
    let released = fixture.snapshot();
    assert_eq!(released.required_bytes, 0);
    assert_eq!(released.unique_bytes, 0);
}
