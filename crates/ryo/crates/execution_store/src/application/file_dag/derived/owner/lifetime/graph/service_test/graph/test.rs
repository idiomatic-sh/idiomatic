//! Real owner handles remain valid/accounted across clear and cannot poison refill.
use super::super::fixtures::{ENDPOINTS, SOURCE, number, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};

fn zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer() {
    let fixture = GraphFixture::new(&[Relation::Function], &[], 0);
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    let first = fixture
        .acquire(SOURCE, &input, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert_eq!(first.sharing(), GraphSharing::Local);
    let live = fixture.snapshot();
    assert_eq!(live.optional_bytes, 0);
    assert_eq!(live.graph_backings, 1);
    let reverse = fixture
        .acquire(SOURCE, &input, [ENDPOINTS[1], ENDPOINTS[0]], &[])
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(first.raw_graph(), reverse.raw_graph()));
    assert_eq!(reverse.orientation(), Orientation::Reverse);
    let shared = fixture.snapshot();
    assert_eq!(shared.unique_bytes, live.unique_bytes);
    assert_eq!(shared.graph_backings, 1);
    assert_eq!(shared.counts.builds, 1);
    let mut walk = GraphWalk::default();
    assert!(
        fixture
            .reach(&reverse, &[], number(Integer(3)), &mut walk)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        walk.reached
            .iter()
            .map(|vertex| reverse.graph().vertices()
                [usize::try_from(vertex.0).expect("a vertex id fits in usize")])
            .collect::<Vec<_>>(),
        [number(Integer(1)), number(Integer(2))]
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

fn old_completed_answer_owns_exact_context_through_clear_and_changed_facts() {
    let fixture = GraphFixture::new(
        &[Relation::Function, Relation::Identifier],
        &[],
        crate::application::file_dag::derived::owner::index::service::LIMIT,
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
            PlanInput::Relation(Relation::Identifier),
            &domain_input,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
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
        .unwrap()
        .unwrap();
    assert_eq!(
        graph.graph().vertices()[usize::try_from(nearest).expect("a vertex id fits in usize")],
        number(Integer(3))
    );
    let answer = fixture
        .reach(&graph, &[], number(Integer(1)), &mut walk)
        .unwrap()
        .unwrap();
    let weak = answer.weak();
    assert_eq!(
        answer
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(Integer(2)), number(Integer(3))]
    );
    let alias = fixture
        .reach(&graph, &[], number(Integer(1)), &mut walk)
        .unwrap()
        .unwrap();
    assert_eq!(answer.data_identity(), alias.data_identity());
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
        .reach(&next, &[], number(Integer(1)), &mut walk)
        .unwrap()
        .unwrap();
    assert_eq!(
        fresh
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(Integer(90))]
    );
    assert_eq!(fixture.snapshot().counts.builds, 1);
    assert_eq!(fixture.snapshot().graph_backings, 2);
    assert_eq!(
        answer
            .values()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [number(Integer(2)), number(Integer(3))]
    );
    assert!(matches!(
        fixture.domain(
            &graph,
            PlanInput::Relation(Relation::Identifier),
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
        [number(Integer(2)), number(Integer(3))]
    );
    assert!(weak.upgrade().is_some());
    drop(answer);
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.snapshot().graph_backings, 0);
    assert_eq!(fixture.snapshot().unique_bytes, 0);
}

fn unavailable_canonical_source_declines_without_retaining_duplicate_graphs() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let input = rows(&columns);
    for (stable, max_views) in [(false, 1024), (true, 0)] {
        let fixture = GraphFixture::with_identity(
            &[Relation::Function],
            &[],
            crate::application::file_dag::derived::owner::index::service::LIMIT,
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

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ZeroOptionalBudgetSharesLiveGraphAndRebuildsOnlyAfterLastOpaqueConsumer,
    OldCompletedAnswerOwnsExactContextThroughClearAndChangedFacts,
    UnavailableCanonicalSourceDeclinesWithoutRetainingDuplicateGraphs,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer",
        ),
        word: Word::Succeeds,
        input: || Scenario::ZeroOptionalBudgetSharesLiveGraphAndRebuildsOnlyAfterLastOpaqueConsumer,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("old_completed_answer_owns_exact_context_through_clear_and_changed_facts"),
        word: Word::Succeeds,
        input: || Scenario::OldCompletedAnswerOwnsExactContextThroughClearAndChangedFacts,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unavailable_canonical_source_declines_without_retaining_duplicate_graphs"),
        word: Word::Succeeds,
        input: || Scenario::UnavailableCanonicalSourceDeclinesWithoutRetainingDuplicateGraphs,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ZeroOptionalBudgetSharesLiveGraphAndRebuildsOnlyAfterLastOpaqueConsumer => {
            zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer();
            Ok(())
        }
        Scenario::OldCompletedAnswerOwnsExactContextThroughClearAndChangedFacts => {
            old_completed_answer_owns_exact_context_through_clear_and_changed_facts();
            Ok(())
        }
        Scenario::UnavailableCanonicalSourceDeclinesWithoutRetainingDuplicateGraphs => {
            unavailable_canonical_source_declines_without_retaining_duplicate_graphs();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
