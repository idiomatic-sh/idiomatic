//! The asks of the `sharing` cases and what each observed.
use crate::application::file_dag::derived::owner::graphs::fixtures::{ENDPOINTS, SOURCE, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use std::borrow::Cow;
use std::sync::Arc;

/// What repeated asks of one source observed.
#[derive(Debug, PartialEq)]
pub(super) struct Repeated {
    pub(super) reached: Cow<'static, [CellValue]>,
    pub(super) first: (u64, u64, u64),
    pub(super) one_graph: bool,
    pub(super) one_answer: bool,
    pub(super) shared: (u64, u64, u64, u64),
    pub(super) reverse: Orientation,
    pub(super) reverse_one_graph: bool,
    pub(super) within_limit: bool,
}

/// What asks under no budget observed.
#[derive(Debug, PartialEq)]
pub(super) struct Unbudgeted {
    pub(super) sharing: GraphSharing,
    pub(super) graphs: usize,
    pub(super) optional_bytes_unchanged: bool,
    pub(super) backings: usize,
    pub(super) answered: bool,
    pub(super) walked: usize,
    pub(super) refusals: u64,
    pub(super) backings_after_drop: usize,
    pub(super) graphless_acquired: bool,
    pub(super) graphless_graphs: usize,
}

/// What asks of unusable populations and changed generations observed.
#[derive(Debug, PartialEq)]
pub(super) struct Unusable {
    pub(super) partial_acquired: bool,
    pub(super) ragged_acquired: bool,
    pub(super) one_graph_across_generations: bool,
    pub(super) builds: u64,
    pub(super) reuses: u64,
    pub(super) same_vertices: bool,
}

/// What asks beside an index observed.
#[derive(Debug, PartialEq)]
pub(super) struct BesideAnIndex {
    pub(super) graph_costs: bool,
    pub(super) indexes: usize,
    pub(super) index_retains_view_bytes: bool,
    pub(super) while_live: GraphSharing,
    pub(super) indexes_while_live: usize,
    pub(super) pinned_refusals: u64,
    pub(super) backings_while_live: usize,
    pub(super) once_dormant: GraphSharing,
    pub(super) indexes_once_dormant: usize,
    pub(super) capacity_kept: bool,
    pub(super) reclaimed_indexes: u64,
    pub(super) retained_view_bytes: usize,
    pub(super) reclaimed_the_view: bool,
    pub(super) optional_within_limit: bool,
    pub(super) one_graph: bool,
    pub(super) same_vertices: bool,
    pub(super) same_forward: bool,
    pub(super) same_backward: bool,
    pub(super) builds: u64,
}

/// What asks after the owner retired its caches observed.
#[derive(Debug, PartialEq)]
pub(super) struct Retired {
    pub(super) graphs: usize,
    pub(super) domains: usize,
    pub(super) required_leased: bool,
    pub(super) graphs_retired: usize,
    pub(super) domains_retired: usize,
    pub(super) retained_view_bytes: usize,
    pub(super) optional_bytes: usize,
    pub(super) held_still_charged: bool,
    pub(super) metadata_released: bool,
    pub(super) only_required_left: bool,
    pub(super) graph_still_read: bool,
    pub(super) domain_still_read: bool,
    pub(super) reach: Option<OperatingError>,
    pub(super) nearest: Option<OperatingError>,
    pub(super) required_after_drop: usize,
    pub(super) unique_after_drop: usize,
}

/// What the asks observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    Repeated(Repeated),
    Unbudgeted(Unbudgeted),
    Unusable(Unusable),
    BesideAnIndex(BesideAnIndex),
    Retired(Retired),
}

pub(super) fn repeated() -> Result<Repeated, OperatingError> {
    let columns = [FactColumn(vec![1, 2, 1, 90]), FactColumn(vec![2, 3, 2, 91])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let first = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let mut walk = GraphWalk::default();
    let actual = fixture
        .reach(&first, &[], CellValue::Number(Integer(1)), &mut walk)?
        .expect("memo admitted");
    let reached = actual
        .iter()
        .map(|vertex| {
            first.graph().vertices()[usize::try_from(*vertex).expect("a vertex id fits in usize")]
        })
        .collect();
    let first_state = fixture.snapshot();
    let second = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let again = fixture
        .reach(&second, &[], CellValue::Number(Integer(1)), &mut walk)?
        .expect("memo hit");
    let shared = fixture.snapshot();
    // Reversing endpoint columns shares adjacency but has a distinct root orientation.
    let reverse = fixture
        .acquire(SOURCE, &rows, [ENDPOINTS[1], ENDPOINTS[0]], &[])?
        .expect("a graph");
    Ok(Repeated {
        reached: Cow::Owned(reached),
        first: (
            first_state.counts.builds,
            first_state.counts.traversals,
            first_state.counts.examined,
        ),
        one_graph: Arc::ptr_eq(first.raw_graph(), second.raw_graph()),
        one_answer: actual.data_identity() == again.data_identity(),
        shared: (
            shared.counts.builds,
            shared.counts.reuses,
            shared.counts.traversals,
            shared.counts.memo_hits,
        ),
        reverse: reverse.orientation(),
        reverse_one_graph: Arc::ptr_eq(first.raw_graph(), reverse.raw_graph()),
        within_limit: fixture.snapshot().bytes <= LIMIT,
    })
}

pub(super) fn unbudgeted() -> Result<Unbudgeted, OperatingError> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function], &[], 0);
    let before = fixture.snapshot();
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let refused = fixture.snapshot();
    let mut walk = GraphWalk::default();
    let answered = fixture
        .reach(&graph, &[], CellValue::Number(Integer(1)), &mut walk)?
        .is_some();
    let refusals = fixture.snapshot().counts.refusals;
    let sharing = graph.sharing();
    drop(graph);
    let backings_after_drop = fixture.snapshot().graph_backings;
    let absent = GraphFixture::new(&[], &[], LIMIT);
    let graphless_acquired = absent.acquire(SOURCE, &rows, ENDPOINTS, &[])?.is_some();
    Ok(Unbudgeted {
        sharing,
        graphs: refused.graphs,
        optional_bytes_unchanged: refused.optional_bytes == before.optional_bytes,
        backings: refused.graph_backings,
        answered,
        walked: walk.reached.len(),
        refusals,
        backings_after_drop,
        graphless_acquired,
        graphless_graphs: absent.snapshot().graphs,
    })
}

pub(super) fn unusable() -> Result<Unusable, OperatingError> {
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
    let partial_acquired = partial.acquire(SOURCE, &source, ENDPOINTS, &[])?.is_some();
    let ragged = [FactColumn(vec![1, 2]), FactColumn(vec![2])];
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let ragged_acquired = fixture
        .acquire(SOURCE, &rows(&ragged), ENDPOINTS, &[])?
        .is_some();
    // The malformed physical population was a distinct file generation.
    fixture.clear();
    let old = fixture
        .acquire(SOURCE, &source, ENDPOINTS, &[])?
        .expect("a graph");
    fixture.clear();
    let changed = [FactColumn(vec![1, 2]), FactColumn(vec![90, 91])];
    let new = fixture
        .acquire(SOURCE, &rows(&changed), ENDPOINTS, &[])?
        .expect("a graph");
    Ok(Unusable {
        partial_acquired,
        ragged_acquired,
        one_graph_across_generations: Arc::ptr_eq(old.raw_graph(), new.raw_graph()),
        builds: fixture.snapshot().counts.builds,
        reuses: fixture.snapshot().counts.reuses,
        same_vertices: old.graph().vertices() == new.graph().vertices(),
    })
}

pub(super) fn beside_an_index() -> Result<BesideAnIndex, OperatingError> {
    use ryo_derived::domain::vertices::types::EdgeDirection;
    let columns = [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![2, 3, 4, 5])];
    let rows = rows(&columns);
    let probe = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let measured = probe
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let graph_cost = probe.graph_cost(&measured);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let index = fixture.index(Relation::Function, &rows, &[ColumnIndex(0)])?;
    let before = fixture.snapshot();
    fixture.set_limit(before.bytes + graph_cost - before.retained_view_bytes);
    let local = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let refused = fixture.snapshot();
    drop(index);
    let shared = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])?
        .expect("a graph");
    let admitted = fixture.snapshot();
    Ok(BesideAnIndex {
        graph_costs: graph_cost > 0,
        indexes: before.indexes,
        index_retains_view_bytes: before.retained_view_bytes > 0,
        while_live: local.sharing(),
        indexes_while_live: refused.indexes,
        pinned_refusals: refused.counts.pinned_refusals,
        backings_while_live: refused.graph_backings,
        once_dormant: shared.sharing(),
        indexes_once_dormant: admitted.indexes,
        capacity_kept: admitted.owner_entries_capacity == before.owner_entries_capacity,
        reclaimed_indexes: admitted.counts.reclaimed_indexes,
        retained_view_bytes: admitted.retained_view_bytes,
        reclaimed_the_view: admitted.counts.reclaimed_bytes
            == u64::try_from(before.retained_view_bytes).expect("a byte count fits in u64"),
        optional_within_limit: admitted.optional_bytes <= admitted.limit,
        one_graph: Arc::ptr_eq(local.raw_graph(), shared.raw_graph()),
        same_vertices: local.graph().vertices() == shared.graph().vertices(),
        same_forward: local.graph().neighbors(EdgeDirection::Forward)
            == shared.graph().neighbors(EdgeDirection::Forward),
        same_backward: local.graph().neighbors(EdgeDirection::Backward)
            == shared.graph().neighbors(EdgeDirection::Backward),
        builds: admitted.counts.builds,
    })
}

pub(super) fn retired() -> Result<Retired, OperatingError> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let edges = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &edges, ENDPOINTS, &[])?
        .expect("a graph");
    let domain_columns = [FactColumn(vec![3]), FactColumn(vec![0])];
    let domain = fixture
        .domain(
            &graph,
            PlanInput::Relation(Relation::Identifier),
            &rows(&domain_columns),
            ColumnIndex(0),
        )?
        .expect("a domain");
    let before = fixture.snapshot();
    let vertices = graph.graph().vertices().to_vec();
    let members = domain.members().0.to_vec();
    fixture.clear();
    let retired = fixture.snapshot();
    let reach = fixture
        .reach(
            &graph,
            &[],
            CellValue::Number(Integer(1)),
            &mut GraphWalk::default(),
        )
        .err();
    let nearest = fixture
        .nearest(
            &graph,
            &domain,
            0,
            Orientation::Forward,
            &mut vec![],
            &mut GraphWalk::default(),
        )
        .err();
    let graph_still_read = graph.graph().vertices() == vertices;
    let domain_still_read = domain.members().0 == members;
    drop(domain);
    drop(graph);
    let released = fixture.snapshot();
    Ok(Retired {
        graphs: before.graphs,
        domains: before.domains,
        required_leased: before.required_bytes > 0,
        graphs_retired: retired.graphs,
        domains_retired: retired.domains,
        retained_view_bytes: retired.retained_view_bytes,
        optional_bytes: retired.optional_bytes,
        held_still_charged: retired.required_bytes > 0,
        metadata_released: retired.required_bytes < before.required_bytes,
        only_required_left: retired.unique_bytes == retired.required_bytes,
        graph_still_read,
        domain_still_read,
        reach,
        nearest,
        required_after_drop: released.required_bytes,
        unique_after_drop: released.unique_bytes,
    })
}
