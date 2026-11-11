//! Generation-local sharing, exact identity and bounded admission through the real owner.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::super::super::fixtures::{ENDPOINTS, SOURCE, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;
use std::sync::Arc;

/// The source a graph is asked over, and what the owner does around the asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The edges 1 -> 2, 2 -> 3 (twice) and 90 -> 91, acquired twice and once reversed, and
    /// reached from 1 each time.
    Repeated,
    /// The edges 1 -> 2 -> 3 under no budget, then under a graph-less owner.
    WithoutBudget,
    /// A partial population, a ragged one, then two generations of different facts.
    Unusable,
    /// The edges 1 -> 2 -> 3 -> 4 -> 5 while an index on its first column is live, then dormant.
    BesideAnIndex,
    /// The edges 1 -> 2 -> 3 and the identifiers {3}, then the owner retires its caches.
    Retired,
}

/// What repeated asks of one source observed.
#[derive(Debug, PartialEq)]
struct Repeated {
    reached: Cow<'static, [CellValue]>,
    first: (u64, u64, u64),
    one_graph: bool,
    one_answer: bool,
    shared: (u64, u64, u64, u64),
    reverse: Orientation,
    reverse_one_graph: bool,
    within_limit: bool,
}

/// What asks under no budget observed.
#[derive(Debug, PartialEq)]
struct Unbudgeted {
    sharing: GraphSharing,
    graphs: usize,
    optional_bytes_unchanged: bool,
    backings: usize,
    answered: bool,
    walked: usize,
    refusals: u64,
    backings_after_drop: usize,
    graphless_acquired: bool,
    graphless_graphs: usize,
}

/// What asks of unusable populations and changed generations observed.
#[derive(Debug, PartialEq)]
struct Unusable {
    partial_acquired: bool,
    ragged_acquired: bool,
    one_graph_across_generations: bool,
    builds: u64,
    reuses: u64,
    same_vertices: bool,
}

/// What asks beside an index observed.
#[derive(Debug, PartialEq)]
struct BesideAnIndex {
    graph_costs: bool,
    indexes: usize,
    index_retains_view_bytes: bool,
    while_live: GraphSharing,
    indexes_while_live: usize,
    pinned_refusals: u64,
    backings_while_live: usize,
    once_dormant: GraphSharing,
    indexes_once_dormant: usize,
    capacity_kept: bool,
    reclaimed_indexes: u64,
    retained_view_bytes: usize,
    reclaimed_the_view: bool,
    optional_within_limit: bool,
    one_graph: bool,
    same_vertices: bool,
    same_forward: bool,
    same_backward: bool,
    builds: u64,
}

/// What asks after the owner retired its caches observed.
#[derive(Debug, PartialEq)]
struct Retired {
    graphs: usize,
    domains: usize,
    required_leased: bool,
    graphs_retired: usize,
    domains_retired: usize,
    retained_view_bytes: usize,
    optional_bytes: usize,
    held_still_charged: bool,
    metadata_released: bool,
    only_required_left: bool,
    graph_still_read: bool,
    domain_still_read: bool,
    reach: Option<OperatingError>,
    nearest: Option<OperatingError>,
    required_after_drop: usize,
    unique_after_drop: usize,
}

/// What the asks observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Repeated(Repeated),
    Unbudgeted(Unbudgeted),
    Unusable(Unusable),
    BesideAnIndex(BesideAnIndex),
    Retired(Retired),
}

type Input = Asked;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared_adjacency_builds_once_and_sparse_root_memo_avoids_retraversal"),
        word: Word::Succeeds,
        input: || Asked::Repeated,
        expect: Expect::Value(Observed::Repeated(Repeated {
            reached: Cow::Borrowed(&[CellValue::Number(Integer(2)), CellValue::Number(Integer(3))]),
            first: (1, 1, 3),
            one_graph: true,
            one_answer: true,
            shared: (1, 1, 1, 1),
            reverse: Orientation::Reverse,
            reverse_one_graph: true,
            within_limit: true,
        })),
    },
    Case {
        name: CaseName("budget_refusal_keeps_complete_local_graph_without_retaining_handles"),
        word: Word::Succeeds,
        input: || Asked::WithoutBudget,
        expect: Expect::Value(Observed::Unbudgeted(Unbudgeted {
            sharing: GraphSharing::Local,
            graphs: 0,
            optional_bytes_unchanged: true,
            backings: 1,
            answered: false,
            walked: 2,
            refusals: 1,
            backings_after_drop: 0,
            graphless_acquired: false,
            graphless_graphs: 0,
        })),
    },
    Case {
        name: CaseName(
            "physical_partiality_ragged_views_and_next_generation_never_hit_old_graphs",
        ),
        word: Word::Succeeds,
        input: || Asked::Unusable,
        expect: Expect::Value(Observed::Unusable(Unusable {
            partial_acquired: false,
            ragged_acquired: false,
            one_graph_across_generations: false,
            builds: 1,
            reuses: 0,
            same_vertices: false,
        })),
    },
    Case {
        name: CaseName(
            "adjacency_admission_reclaims_dormant_payload_but_falls_back_while_the_index_is_live",
        ),
        word: Word::Succeeds,
        input: || Asked::BesideAnIndex,
        expect: Expect::Value(Observed::BesideAnIndex(BesideAnIndex {
            graph_costs: true,
            indexes: 1,
            index_retains_view_bytes: true,
            while_live: GraphSharing::Local,
            indexes_while_live: 1,
            pinned_refusals: 1,
            backings_while_live: 1,
            once_dormant: GraphSharing::Shared,
            indexes_once_dormant: 0,
            capacity_kept: true,
            reclaimed_indexes: 1,
            retained_view_bytes: 0,
            reclaimed_the_view: true,
            optional_within_limit: true,
            one_graph: true,
            same_vertices: true,
            same_forward: true,
            same_backward: true,
            builds: 1,
        })),
    },
    Case {
        name: CaseName(
            "retired_immutable_handles_keep_their_lease_but_cannot_authorize_new_demands",
        ),
        word: Word::Succeeds,
        input: || Asked::Retired,
        expect: Expect::Value(Observed::Retired(Retired {
            graphs: 1,
            domains: 1,
            required_leased: true,
            graphs_retired: 0,
            domains_retired: 0,
            retained_view_bytes: 0,
            optional_bytes: 0,
            held_still_charged: true,
            metadata_released: true,
            only_required_left: true,
            graph_still_read: true,
            domain_still_read: true,
            reach: Some(OperatingError::NotRead),
            nearest: Some(OperatingError::NotRead),
            required_after_drop: 0,
            unique_after_drop: 0,
        })),
    },
];

/// Asks the owner for the graph as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Asked::Repeated => Observed::Repeated(repeated()?),
        Asked::WithoutBudget => Observed::Unbudgeted(unbudgeted()?),
        Asked::Unusable => Observed::Unusable(unusable()?),
        Asked::BesideAnIndex => Observed::BesideAnIndex(beside_an_index()?),
        Asked::Retired => Observed::Retired(retired()?),
    })
}

fn repeated() -> Result<Repeated, OperatingError> {
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

fn unbudgeted() -> Result<Unbudgeted, OperatingError> {
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

fn unusable() -> Result<Unusable, OperatingError> {
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

fn beside_an_index() -> Result<BesideAnIndex, OperatingError> {
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

fn retired() -> Result<Retired, OperatingError> {
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
