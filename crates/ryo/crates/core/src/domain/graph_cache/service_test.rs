//! Generation-local sharing, exact identity and bounded admission contracts.

use super::service::{acquire, domain};
use super::types::roots::value::DemandAnswers;
use crate::domain::stored_graph::types::{GraphWalk, Orientation};
use crate::domain::stored_indexes::service::{LIMIT, new};
use crate::domain::table_model::types::{Rows, SelectedRows, SelectionIdentity, StoredSelection};
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;

const SOURCE: Input = Input::Relation(Relation::Function);

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
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    let first = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("graph");
    let mut walk = GraphWalk::default();
    let actual = DemandAnswers::new(Some(&mut cache))
        .reach(&first, &[], Value::Number(Integer(1)), &mut walk)
        .expect("memo admitted");
    let values: Vec<_> = actual
        .iter()
        .map(|vertex| first.graph.vertices[*vertex as usize])
        .collect();
    assert_eq!(
        values,
        [Value::Number(Integer(2)), Value::Number(Integer(3))]
    );
    assert_eq!(
        (
            cache.graph_counts.builds,
            cache.graph_counts.traversals,
            cache.graph_counts.examined
        ),
        (1, 1, 3)
    );
    let second = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("graph");
    assert!(Arc::ptr_eq(&first.graph, &second.graph));
    let again = DemandAnswers::new(Some(&mut cache))
        .reach(&second, &[], Value::Number(Integer(1)), &mut walk)
        .expect("memo hit");
    assert!(Arc::ptr_eq(&actual, &again));
    assert_eq!(
        (
            cache.graph_counts.builds,
            cache.graph_counts.reuses,
            cache.graph_counts.traversals,
            cache.graph_counts.memo_hits
        ),
        (1, 1, 1, 1)
    );
    // Reversing endpoint columns shares the same adjacency but not root answers.
    let reverse = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(1), ColumnIndex(0)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("reverse");
    assert_eq!(reverse.orientation, Orientation::Reverse);
    assert!(Arc::ptr_eq(&first.graph, &reverse.graph));
    assert!(cache.bytes <= LIMIT);
}

#[test]
fn exact_selected_bindings_population_and_projection_determine_graph_identity() {
    let columns = [FactColumn(vec![1, 2, 90]), FactColumn(vec![2, 3, 91])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function, Relation::Identifier], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
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
    let original = acquire(
        SOURCE,
        &first,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("graph");
    let reordered = selection(1, vec![0, 1], vec![ColumnIndex(1), ColumnIndex(0)]);
    let same = acquire(
        SOURCE,
        &reordered,
        [ColumnIndex(1), ColumnIndex(0)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("graph");
    assert!(Arc::ptr_eq(&original.graph, &same.graph));
    for different in [
        selection(2, vec![0, 1], vec![ColumnIndex(0), ColumnIndex(1)]),
        selection(1, vec![0, 2], vec![ColumnIndex(0), ColumnIndex(1)]),
    ] {
        let graph = acquire(
            SOURCE,
            &different,
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
            Some(&mut cache),
        )
        .expect("columns")
        .expect("different graph");
        assert!(!Arc::ptr_eq(&original.graph, &graph.graph));
    }
    let other = acquire(
        Input::Relation(Relation::Identifier),
        &Rows::Stored(source),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("relation");
    assert!(!Arc::ptr_eq(&original.graph, &other.graph));
    assert_eq!(cache.graph_counts.builds, 4);
}

#[test]
fn budget_refusal_keeps_complete_local_graph_without_retaining_handles() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let limit =
        size_of::<crate::domain::stored_indexes::types::FileIndexes>() + size_of::<Relation>() + 8;
    let mut cache = new(&[Relation::Function], &[], limit).expect("metadata fits");
    cache.graph_enabled = true;
    let before = cache.bytes;
    let graph = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("local graph");
    assert!(!graph.shared);
    assert!(cache.graphs.is_empty());
    assert_eq!(cache.bytes, before);
    let mut walk = GraphWalk::default();
    assert!(
        DemandAnswers::new(Some(&mut cache))
            .reach(&graph, &[], Value::Number(Integer(1)), &mut walk)
            .is_none()
    );
    assert_eq!(walk.reached.len(), 2);
    assert_eq!(cache.graph_counts.refusals, 1);
    assert!(cache.bytes <= limit);
    assert!(
        acquire(SOURCE, &rows, [ColumnIndex(0), ColumnIndex(1)], &[], None)
            .expect("fallback")
            .is_none()
    );
    let mut next = new(&[], &[], LIMIT).expect("next generation");
    next.graph_enabled = true;
    assert!(
        acquire(
            SOURCE,
            &rows,
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
            Some(&mut next)
        )
        .expect("ineligible")
        .is_none()
    );
    assert!(next.graphs.is_empty());
}

#[test]
fn nearest_memo_is_shared_only_for_exact_domain_and_generation() {
    let columns = [FactColumn(vec![1, 2, 8]), FactColumn(vec![2, 3, 2])];
    let rows = rows(&columns);
    let mut cache = new(&[Relation::Function, Relation::Identifier], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    let graph = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("graph");
    let domain_columns = [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])];
    let domain_rows = self::rows(&domain_columns);
    let membership = domain(
        &graph,
        Input::Relation(Relation::Identifier),
        &domain_rows,
        ColumnIndex(0),
        Some(&mut cache),
    )
    .expect("domain");
    let again = domain(
        &graph,
        Input::Relation(Relation::Identifier),
        &domain_rows,
        ColumnIndex(0),
        Some(&mut cache),
    )
    .expect("domain");
    assert!(Arc::ptr_eq(&membership, &again));
    let root = graph.graph.vertex_ids[&(0, Value::Number(Integer(1)))];
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    let first = DemandAnswers::new(Some(&mut cache)).nearest(
        &graph,
        &membership,
        root,
        Orientation::Forward,
        &mut local,
        &mut walk,
    );
    assert_eq!(
        first.map(|vertex| graph.graph.vertices[vertex as usize]),
        Some(Value::Number(Integer(3)))
    );
    assert_eq!(cache.nearest.len(), 1);
    assert_eq!(
        DemandAnswers::new(Some(&mut cache)).nearest(
            &graph,
            &again,
            root,
            Orientation::Forward,
            &mut local,
            &mut walk
        ),
        first
    );
    assert_eq!(cache.graph_counts.forest_hits, 1);
    // Computed domains cannot borrow the stable relation's membership identity.
    let table = crate::domain::table_model::types::Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(2))],
        count: crate::domain::table_model::types::RowCount(1),
    };
    let different = domain(
        &graph,
        Input::Relation(Relation::Identifier),
        &Rows::Computed(&table),
        ColumnIndex(0),
        Some(&mut cache),
    )
    .expect("local domain");
    local.clear();
    let changed = DemandAnswers::new(Some(&mut cache)).nearest(
        &graph,
        &different,
        root,
        Orientation::Forward,
        &mut local,
        &mut walk,
    );
    assert_eq!(
        changed.map(|vertex| graph.graph.vertices[vertex as usize]),
        Some(Value::Number(Integer(2)))
    );
    assert_eq!(cache.nearest.len(), 1);
    assert!(cache.memo_bytes <= super::service::MEMO_LIMIT);
    assert!(cache.bytes <= LIMIT);
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
    let mut partial = new(&[Relation::Function], &partials, LIMIT).expect("partial cache");
    partial.graph_enabled = true;
    assert!(
        acquire(
            SOURCE,
            &source,
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
            Some(&mut partial)
        )
        .expect("fallback")
        .is_none()
    );
    let ragged = [FactColumn(vec![1, 2]), FactColumn(vec![2])];
    let mut first = new(&[Relation::Function], &[], LIMIT).expect("cache");
    first.graph_enabled = true;
    assert!(
        acquire(
            SOURCE,
            &rows(&ragged),
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
            Some(&mut first)
        )
        .expect("ragged fallback")
        .is_none()
    );
    let old = acquire(
        SOURCE,
        &source,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut first),
    )
    .expect("columns")
    .expect("old graph");
    drop(first);
    let changed = [FactColumn(vec![1, 2]), FactColumn(vec![90, 91])];
    let mut next = new(&[Relation::Function], &[], LIMIT).expect("next visit, even same file id");
    next.graph_enabled = true;
    let new = acquire(
        SOURCE,
        &rows(&changed),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut next),
    )
    .expect("columns")
    .expect("new graph");
    assert!(!Arc::ptr_eq(&old.graph, &new.graph));
    assert_eq!(next.graph_counts.builds, 1);
    assert_eq!(next.graph_counts.reuses, 0);
    assert_ne!(old.graph.vertices, new.graph.vertices);
}

#[test]
fn adjacency_admission_reclaims_dormant_payload_but_falls_back_while_the_index_is_live() {
    use crate::domain::graph_cache::types::reclamation::value::index_payload;
    use crate::domain::stored_graph::types::footprint::value::GraphFootprint;
    use crate::domain::stored_indexes::service::acquire as acquire_index;
    use crate::domain::table_model::types::KeyIndex;
    let columns = [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![2, 3, 4, 5])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let rows = Rows::Stored(source);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    let handle = acquire_index(
        &mut cache,
        Relation::Function,
        source,
        &[ColumnIndex(0)],
        &mut KeyIndex::default(),
    )
    .expect("ordinary index")
    .shared
    .expect("retained index");
    let credit = index_payload(&cache.entries[0]).expect("payload");
    let metadata_capacity = cache.entries.capacity();
    let graph_source = crate::domain::stored_graph::service::source(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        &cache.eligible,
    )
    .expect("identity");
    let graph = crate::domain::stored_graph::service::build(
        graph_source.identity,
        &rows,
        graph_source.endpoints,
        &[],
    )
    .expect("columns")
    .expect("graph");
    let payload = GraphFootprint::new(&cache)
        .graph(&graph)
        .expect("graph capacity");
    let (_, metadata) = super::service::slot(&cache.graphs).expect("slot capacity");
    cache.limit = cache.bytes + payload + metadata - credit / 2;
    let before = cache.bytes;
    let local = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("complete fallback");
    assert!(!local.shared);
    assert_eq!(cache.bytes, before);
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.graph_counts.pinned_refusals, 1);
    drop(handle);
    let shared = acquire(
        SOURCE,
        &rows,
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
        Some(&mut cache),
    )
    .expect("columns")
    .expect("admitted graph");
    assert!(shared.shared);
    assert!(cache.entries.is_empty());
    assert_eq!(cache.entries.capacity(), metadata_capacity);
    assert_eq!(cache.graph_counts.reclaimed_indexes, 1);
    assert!(cache.bytes <= cache.limit);
    assert_eq!(local.graph.vertices, shared.graph.vertices);
    assert_eq!(local.graph.neighbors, shared.graph.neighbors);
    assert_eq!(cache.graph_counts.builds, 2);
}
