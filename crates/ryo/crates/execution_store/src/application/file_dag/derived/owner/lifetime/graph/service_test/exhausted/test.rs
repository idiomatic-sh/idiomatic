//! An exhausted domain identity keeps local membership without retained duplicates.
use super::super::fixtures::{ENDPOINTS, SOURCE, number, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
fn exhausted_domain_identity_uses_local_membership_without_retained_duplicates() {
    let fixture = GraphFixture::with_identity(
        &[Relation::Function, Relation::Identifier],
        &[],
        crate::application::file_dag::derived::owner::index::service::LIMIT,
        true,
        1,
    );
    let edges = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let graph = fixture
        .acquire(SOURCE, &rows(&edges), ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert_eq!(graph.sharing(), GraphSharing::Shared);
    let domain_columns = [FactColumn(vec![2, 3]), FactColumn(vec![0, 0])];
    let input = rows(&domain_columns);
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    for _ in 0..2 {
        let domain = fixture
            .domain(
                &graph,
                PlanInput::Relation(Relation::Identifier),
                &input,
                ColumnIndex(0),
            )
            .unwrap()
            .unwrap();
        assert!(
            domain.raw_domain().identity.is_none(),
            "an exhausted source publication cannot grant stable membership identity"
        );
        let nearest = fixture
            .nearest(
                &graph,
                &domain,
                graph
                    .graph()
                    .vertex(
                        ryo_derived::domain::vertices::types::PartitionId(0),
                        number(Integer(1)),
                    )
                    .unwrap()
                    .0,
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .unwrap();
        assert_eq!(
            nearest.map(|at| graph.graph().vertices()[usize::try_from(at).unwrap()]),
            Some(number(Integer(2)))
        );
    }
    let observed = fixture.snapshot();
    assert_eq!(observed.graphs, 1);
    assert_eq!(observed.domains, 0);
    assert_eq!(observed.nearest, 0);
    assert_eq!(observed.counts.builds, 1);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ExhaustedDomainIdentityUsesLocalMembershipWithoutRetainedDuplicates,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("exhausted_domain_identity_uses_local_membership_without_retained_duplicates"),
    word: Word::Succeeds,
    input: || Scenario::ExhaustedDomainIdentityUsesLocalMembershipWithoutRetainedDuplicates,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ExhaustedDomainIdentityUsesLocalMembershipWithoutRetainedDuplicates => {
            exhausted_domain_identity_uses_local_membership_without_retained_duplicates();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
