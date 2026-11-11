//! Real owner handles remain valid/accounted across clear and cannot poison refill.
use super::super::fixtures::{ENDPOINTS, SOURCE, number, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::population::service::PopulationId;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::application::file_dag::derived::owner::footprint::service::storage_bytes;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::DerivedContract, *},
    operating::errors::OperatingError,
};
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};
use crate::application::file_dag::derived::owner::types::{
    AdmissionOrder, BuildCount, BuildCounts, EntryLimit, FilePreparation, HitCount,
    IdentityCompatibility, MembershipOrder, MemoBudget, PinCount, PinState, PopulationCount,
    ReclaimOutlook, RefusalCount, ReleasedEntries, Room, ScopeId, ViewCount,
};
use crate::application::file_dag::derived::owner::graph_key::{GraphRequirement, GraphViewIdentity, RequirementIdentity};
use ryo_derived::domain::request::types::DerivedKind;
use crate::application::file_dag::derived::owner::service::*;
use crate::application::file_dag::derived::owner::types::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use std::borrow::Cow;

/// How a graph's consumers live: the budget and source identity the owner runs under.
#[derive(Clone, Copy, Debug)]
enum Lifetime {
    /// No optional budget: a forward and a reverse consumer share one live graph.
    NoOptionalBudget,
    /// A completed reach answer outlives a clear and a changed source.
    CompletedAnswer,
    /// Each pair is a source identity's stability and view limit the owner cannot identify.
    WithoutCanonicalIdentity([(bool, usize); 2]),
}

/// The graph backings, builds and optional bytes the owner holds at one point.
#[derive(Debug, PartialEq)]
struct Held {
    backings: usize,
    builds: u64,
    optional_bytes: usize,
}

/// What a forward and a reverse consumer of one graph observed, and what the owner held as they
/// dropped and a next consumer acquired.
#[derive(Debug, PartialEq)]
struct Shared {
    sharing: GraphSharing,
    live: Held,
    one_backing: bool,
    reverse: Orientation,
    unique_bytes_unchanged: bool,
    shared: Held,
    reverse_answered: bool,
    reverse_reached: Cow<'static, [Cell]>,
    after_first_dropped: usize,
    expired: Held,
    refilled: Held,
    after_refill_dropped: usize,
}

/// What a completed answer observed through a clear and a changed source.
#[derive(Debug, PartialEq)]
struct Completed {
    nearest: Cell,
    answer: Cow<'static, [Cell]>,
    alias_same_data: bool,
    live_backings: usize,
    unique_is_required_plus_optional_less_overlap: bool,
    cleared_optional_bytes: usize,
    cleared_backings: usize,
    fresh: Cow<'static, [Cell]>,
    changed_builds: u64,
    changed_backings: usize,
    answer_after_change: Cow<'static, [Cell]>,
    old_domain_refused: Option<OperatingError>,
    alone: Cow<'static, [Cell]>,
    alone_backings: usize,
    alive_before_drop: bool,
    alive_after_drop: bool,
    dropped_backings: usize,
    dropped_unique_bytes: usize,
}

/// Per identity: whether each of two acquisitions answered a graph, and what the owner held.
#[derive(Debug, PartialEq)]
struct Declined {
    acquired: [bool; 2],
    graphs: usize,
    held: Held,
}

/// What a lifetime observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Shared(Shared),
    Completed(Completed),
    Declined([Declined; 2]),
}

type Input = Lifetime;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer",
        ),
        word: Word::Succeeds,
        input: || Lifetime::NoOptionalBudget,
        expect: Expect::Value(Observed::Shared(Shared {
            sharing: GraphSharing::Local,
            live: Held {
                backings: 1,
                builds: 1,
                optional_bytes: 0,
            },
            one_backing: true,
            reverse: Orientation::Reverse,
            unique_bytes_unchanged: true,
            shared: Held {
                backings: 1,
                builds: 1,
                optional_bytes: 0,
            },
            reverse_answered: false,
            reverse_reached: Cow::Borrowed(&[Cell::Number(Integer(1)), Cell::Number(Integer(2))]),
            after_first_dropped: 1,
            expired: Held {
                backings: 0,
                builds: 1,
                optional_bytes: 0,
            },
            refilled: Held {
                backings: 1,
                builds: 2,
                optional_bytes: 0,
            },
            after_refill_dropped: 0,
        })),
    },
    Case {
        name: CaseName("old_completed_answer_owns_exact_context_through_clear_and_changed_facts"),
        word: Word::Succeeds,
        input: || Lifetime::CompletedAnswer,
        expect: Expect::Value(Observed::Completed(Completed {
            nearest: Cell::Number(Integer(3)),
            answer: Cow::Borrowed(&[Cell::Number(Integer(2)), Cell::Number(Integer(3))]),
            alias_same_data: true,
            live_backings: 1,
            unique_is_required_plus_optional_less_overlap: true,
            cleared_optional_bytes: 0,
            cleared_backings: 1,
            fresh: Cow::Borrowed(&[Cell::Number(Integer(90))]),
            changed_builds: 1,
            changed_backings: 2,
            answer_after_change: Cow::Borrowed(&[
                Cell::Number(Integer(2)),
                Cell::Number(Integer(3)),
            ]),
            old_domain_refused: Some(OperatingError::NotRead),
            alone: Cow::Borrowed(&[Cell::Number(Integer(2)), Cell::Number(Integer(3))]),
            alone_backings: 1,
            alive_before_drop: true,
            alive_after_drop: false,
            dropped_backings: 0,
            dropped_unique_bytes: 0,
        })),
    },
    Case {
        name: CaseName("unavailable_canonical_source_declines_without_retaining_duplicate_graphs"),
        word: Word::Succeeds,
        input: || Lifetime::WithoutCanonicalIdentity([(false, 1024), (true, 0)]),
        expect: Expect::Value(Observed::Declined([
            Declined {
                acquired: [false, false],
                graphs: 0,
                held: Held {
                    backings: 0,
                    builds: 0,
                    optional_bytes: 0,
                },
            },
            Declined {
                acquired: [false, false],
                graphs: 0,
                held: Held {
                    backings: 0,
                    builds: 0,
                    optional_bytes: 0,
                },
            },
        ])),
    },
];

fn held(fixture: &GraphFixture) -> Held {
    let snapshot = fixture.snapshot();
    Held {
        backings: snapshot.graph_backings,
        builds: snapshot.counts.builds,
        optional_bytes: snapshot.optional_bytes,
    }
}

fn reached(answer: &graphs::types::handles::service::RootAnswer) -> Cow<'static, [Cell]> {
    Cow::Owned(
        answer
            .values()
            .expect("a completed answer reads its values")
            .collect::<Result<Vec<_>, _>>()
            .expect("every reached vertex decodes"),
    )
}

/// Runs the lifetime against an owner over the edges 1 -> 2 -> 3.
fn subject(input: Input) -> Result<Value, Failure> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let edges = rows(&columns);
    Ok(match input {
        Lifetime::NoOptionalBudget => Observed::Shared(shared(&edges)),
        Lifetime::CompletedAnswer => Observed::Completed(completed(&edges)),
        Lifetime::WithoutCanonicalIdentity(identities) => Observed::Declined(identities.map(
            |(stable, max_views)| {
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
            },
        )),
    })
}

fn shared(edges: &Rows<'_>) -> Shared {
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

fn completed(edges: &Rows<'_>) -> Completed {
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
