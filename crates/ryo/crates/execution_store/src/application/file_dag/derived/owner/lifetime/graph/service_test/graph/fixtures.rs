//! The support of the `graph` cases.
use super::super::fixtures::{ENDPOINTS, SOURCE, number, rows};
use crate::application::file_dag::derived::owner::graphs::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use std::borrow::Cow;
use std::sync::Arc;

/// How a graph's consumers live: the budget and source identity the owner runs under.
#[derive(Clone, Copy, Debug)]
pub(super) enum Lifetime {
    /// No optional budget: a forward and a reverse consumer share one live graph.
    NoOptionalBudget,
    /// A completed reach answer outlives a clear and a changed source.
    CompletedAnswer,
    /// Each pair is a source identity's stability and view limit the owner cannot identify.
    WithoutCanonicalIdentity([(bool, usize); 2]),
}

/// The graph backings, builds and optional bytes the owner holds at one point.
#[derive(Debug, PartialEq)]
pub(super) struct Held {
    pub(super) backings: usize,
    pub(super) builds: u64,
    pub(super) optional_bytes: usize,
}

/// What a forward and a reverse consumer of one graph observed, and what the owner held as they
/// dropped and a next consumer acquired.
#[derive(Debug, PartialEq)]
pub(super) struct Shared {
    pub(super) sharing: GraphSharing,
    pub(super) live: Held,
    pub(super) one_backing: bool,
    pub(super) reverse: Orientation,
    pub(super) unique_bytes_unchanged: bool,
    pub(super) shared: Held,
    pub(super) reverse_answered: bool,
    pub(super) reverse_reached: Cow<'static, [Cell]>,
    pub(super) after_first_dropped: usize,
    pub(super) expired: Held,
    pub(super) refilled: Held,
    pub(super) after_refill_dropped: usize,
}

/// What a completed answer observed through a clear and a changed source.
#[derive(Debug, PartialEq)]
pub(super) struct Completed {
    pub(super) nearest: Cell,
    pub(super) answer: Cow<'static, [Cell]>,
    pub(super) alias_same_data: bool,
    pub(super) live_backings: usize,
    pub(super) unique_is_required_plus_optional_less_overlap: bool,
    pub(super) cleared_optional_bytes: usize,
    pub(super) cleared_backings: usize,
    pub(super) fresh: Cow<'static, [Cell]>,
    pub(super) changed_builds: u64,
    pub(super) changed_backings: usize,
    pub(super) answer_after_change: Cow<'static, [Cell]>,
    pub(super) old_domain_refused: Option<OperatingError>,
    pub(super) alone: Cow<'static, [Cell]>,
    pub(super) alone_backings: usize,
    pub(super) alive_before_drop: bool,
    pub(super) alive_after_drop: bool,
    pub(super) dropped_backings: usize,
    pub(super) dropped_unique_bytes: usize,
}

/// Per identity: whether each of two acquisitions answered a graph, and what the owner held.
#[derive(Debug, PartialEq)]
pub(super) struct Declined {
    pub(super) acquired: [bool; 2],
    pub(super) graphs: usize,
    pub(super) held: Held,
}

/// What a lifetime observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    Shared(Shared),
    Completed(Completed),
    Declined([Declined; 2]),
}

pub(super) fn held(fixture: &GraphFixture) -> Held {
    let snapshot = fixture.snapshot();
    Held {
        backings: snapshot.graph_backings,
        builds: snapshot.counts.builds,
        optional_bytes: snapshot.optional_bytes,
    }
}

pub(super) fn reached(answer: &graphs::handles::service::RootAnswer) -> Cow<'static, [Cell]> {
    Cow::Owned(
        answer
            .values()
            .expect("a completed answer reads its values")
            .collect::<Result<Vec<_>, _>>()
            .expect("every reached vertex decodes"),
    )
}

/// Runs the lifetime against an owner over the edges 1 -> 2 -> 3.
pub(super) fn observe(input: Lifetime) -> Result<Observed, String> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let edges = rows(&columns);
    Ok(match input {
        Lifetime::NoOptionalBudget => Observed::Shared(shared(&edges)),
        Lifetime::CompletedAnswer => Observed::Completed(completed(&edges)),
        Lifetime::WithoutCanonicalIdentity(identities) => {
            Observed::Declined(identities.map(|(stable, max_views)| {
                let fixture = GraphFixture::with_identity(
                    &[Relation::Function],
                    &[],
                    index::service::LIMIT,
                    stable,
                    max_views,
                );
                let acquired = [(); 2].map(|()| {
                    fixture
                        .acquire(SOURCE, &edges, ENDPOINTS, &[])
                        .expect("valid columns")
                        .is_some()
                });
                Declined {
                    acquired,
                    graphs: fixture.snapshot().graphs,
                    held: held(&fixture),
                }
            }))
        }
    })
}

pub(super) fn shared(edges: &Rows<'_>) -> Shared {
    let fixture = GraphFixture::new(&[Relation::Function], &[], 0);
    let first = fixture
        .acquire(SOURCE, edges, ENDPOINTS, &[])
        .expect("valid columns")
        .expect("a graph");
    let live_unique = fixture.snapshot().unique_bytes;
    let live = held(&fixture);
    let reverse = fixture
        .acquire(SOURCE, edges, [ENDPOINTS[1], ENDPOINTS[0]], &[])
        .expect("valid columns")
        .expect("a graph");
    let one_backing = Arc::ptr_eq(first.raw_graph(), reverse.raw_graph());
    let unique_bytes_unchanged = fixture.snapshot().unique_bytes == live_unique;
    let shared = held(&fixture);
    let mut walk = GraphWalk::default();
    let reverse_answered = fixture
        .reach(&reverse, &[], number(Integer(3)), &mut walk)
        .expect("a valid root query")
        .is_some();
    let reverse_reached = Cow::Owned(
        walk.reached
            .iter()
            .map(|vertex| {
                reverse.graph().vertices()
                    [usize::try_from(vertex.0).expect("a vertex id fits in usize")]
            })
            .collect(),
    );
    let sharing = first.sharing();
    let orientation = reverse.orientation();
    drop(first);
    let after_first_dropped = fixture.snapshot().graph_backings;
    drop(reverse);
    let expired = held(&fixture);
    let next = fixture
        .acquire(SOURCE, edges, ENDPOINTS, &[])
        .expect("valid columns")
        .expect("a graph");
    let refilled = held(&fixture);
    drop(next);
    Shared {
        sharing,
        live,
        one_backing,
        reverse: orientation,
        unique_bytes_unchanged,
        shared,
        reverse_answered,
        reverse_reached,
        after_first_dropped,
        expired,
        refilled,
        after_refill_dropped: fixture.snapshot().graph_backings,
    }
}

pub(super) fn completed(edges: &Rows<'_>) -> Completed {
    let fixture = GraphFixture::new(
        &[Relation::Function, Relation::Identifier],
        &[],
        index::service::LIMIT,
    );
    let graph = fixture
        .acquire(SOURCE, edges, ENDPOINTS, &[])
        .expect("valid columns")
        .expect("a graph");
    let domain_columns = [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])];
    let domain_input = rows(&domain_columns);
    let domain = fixture
        .domain(
            &graph,
            PlanInput::Relation(Relation::Identifier),
            &domain_input,
            ColumnIndex(0),
        )
        .expect("valid domain columns")
        .expect("a domain");
    let mut walk = GraphWalk::default();
    let root = graph
        .graph()
        .vertex(
            ryo_derived::domain::vertices::types::PartitionId(0),
            number(Integer(1)),
        )
        .expect("prepared vertex")
        .0;
    let nearest = fixture
        .nearest(
            &graph,
            &domain,
            root,
            Orientation::Forward,
            &mut vec![],
            &mut walk,
        )
        .expect("a valid nearest query")
        .expect("a nearest vertex");
    let nearest =
        graph.graph().vertices()[usize::try_from(nearest).expect("a vertex id fits in usize")];
    let answer = fixture
        .reach(&graph, &[], number(Integer(1)), &mut walk)
        .expect("a valid root query")
        .expect("a reach answer");
    let weak = answer.weak();
    let answered = reached(&answer);
    let alias = fixture
        .reach(&graph, &[], number(Integer(1)), &mut walk)
        .expect("a valid root query")
        .expect("a reach answer");
    let alias_same_data = answer.data_identity() == alias.data_identity();
    let live = fixture.snapshot();
    drop(alias);
    fixture.clear();
    let cleared = held(&fixture);
    let changed_columns = [FactColumn(vec![1, 2]), FactColumn(vec![90, 91])];
    let next_input = rows(&changed_columns);
    let next = fixture
        .acquire(SOURCE, &next_input, ENDPOINTS, &[])
        .expect("valid columns")
        .expect("a graph");
    let fresh = fixture
        .reach(&next, &[], number(Integer(1)), &mut walk)
        .expect("a valid root query")
        .expect("a reach answer");
    let fresh_reached = reached(&fresh);
    let changed = held(&fixture);
    let answer_after_change = reached(&answer);
    let old_domain_refused = fixture
        .domain(
            &graph,
            PlanInput::Relation(Relation::Identifier),
            &domain_input,
            ColumnIndex(0),
        )
        .err();
    drop(graph);
    drop(domain);
    drop(next);
    drop(fresh);
    fixture.clear();
    let alone_backings = fixture.snapshot().graph_backings;
    let alone = reached(&answer);
    let alive_before_drop = weak.upgrade().is_some();
    drop(answer);
    Completed {
        nearest,
        answer: answered,
        alias_same_data,
        live_backings: live.graph_backings,
        unique_is_required_plus_optional_less_overlap: live.unique_bytes
            == live.required_bytes + live.optional_bytes - live.overlap_bytes,
        cleared_optional_bytes: cleared.optional_bytes,
        cleared_backings: cleared.backings,
        fresh: fresh_reached,
        changed_builds: changed.builds,
        changed_backings: changed.backings,
        answer_after_change,
        old_domain_refused,
        alone,
        alone_backings,
        alive_before_drop,
        alive_after_drop: weak.upgrade().is_some(),
        dropped_backings: fixture.snapshot().graph_backings,
        dropped_unique_bytes: fixture.snapshot().unique_bytes,
    }
}
