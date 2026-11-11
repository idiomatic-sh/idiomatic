//! An exhausted domain identity keeps local membership without retained duplicates.
use super::super::fixtures::{ENDPOINTS, SOURCE, number, rows};
use crate::application::file_dag::derived::owner::graphs::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};

/// The edges a graph is acquired over, the domain rows asked for, and the vertex each of the two
/// walks starts at.
#[derive(Debug)]
struct Walks {
    edges: [&'static [u64]; 2],
    domain: [&'static [u64]; 2],
    from: i64,
}

/// What the two walks observed: the graph's sharing, per walk whether the domain carried a membership
/// identity and the nearest vertex reached, and what the owner retained.
#[derive(Debug, PartialEq)]
struct Observed {
    sharing: GraphSharing,
    identities: [bool; 2],
    nearest: [Option<Cell>; 2],
    graphs: usize,
    domains: usize,
    retained_nearest: usize,
    builds: u64,
}

type Input = Walks;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("exhausted_domain_identity_uses_local_membership_without_retained_duplicates"),
    word: Word::Succeeds,
    input: || Walks {
        edges: [&[1, 2], &[2, 3]],
        domain: [&[2, 3], &[0, 0]],
        from: 1,
    },
    expect: Expect::Value(Observed {
        sharing: GraphSharing::Shared,
        identities: [false, false],
        nearest: [
            Some(Cell::Number(Integer(2))),
            Some(Cell::Number(Integer(2))),
        ],
        graphs: 1,
        domains: 0,
        retained_nearest: 0,
        builds: 1,
    }),
}];

/// Acquires the graph under an exhausted source identity, asks for the domain and walks it
/// twice, and reads what the owner retained.
fn subject(input: Input) -> Result<Value, Failure> {
    let fixture = GraphFixture::with_identity(
        &[Relation::Function, Relation::Identifier],
        &[],
        index::service::LIMIT,
        true,
        1,
    );
    let edges = input.edges.map(|column| FactColumn(column.to_vec()));
    let graph = fixture
        .acquire(SOURCE, &rows(&edges), ENDPOINTS, &[])
        .map_err(|error| format!("{error:?}"))?
        .ok_or("no graph")?;
    let domain_columns = input.domain.map(|column| FactColumn(column.to_vec()));
    let domain_rows = rows(&domain_columns);
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    let mut identities = [true; 2];
    let mut nearest = [None; 2];
    for pass in 0..2 {
        let domain = fixture
            .domain(
                &graph,
                PlanInput::Relation(Relation::Identifier),
                &domain_rows,
                ColumnIndex(0),
            )
            .map_err(|error| format!("{error:?}"))?
            .ok_or("no domain")?;
        identities[pass] = domain.raw_domain().identity.is_some();
        let start = graph
            .graph()
            .vertex(
                ryo_derived::domain::vertices::types::PartitionId(0),
                number(Integer(input.from)),
            )
            .ok_or("no start vertex")?
            .0;
        let at = fixture
            .nearest(
                &graph,
                &domain,
                start,
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .map_err(|error| format!("{error:?}"))?;
        nearest[pass] = at.map(|at| graph.graph().vertices()[usize::try_from(at).unwrap()]);
    }
    let observed = fixture.snapshot();
    Ok(Observed {
        sharing: graph.sharing(),
        identities,
        nearest,
        graphs: observed.graphs,
        domains: observed.domains,
        retained_nearest: observed.nearest,
        builds: observed.counts.builds,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
