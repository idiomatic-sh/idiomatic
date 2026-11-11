//! Nearest-ancestor answers through the shared derived forest, with exact domain and generation identity.
use crate::cases::{Case, CaseName, Expect, Word, run};

use crate::application::file_dag::derived::owner::graphs::fixtures::{ENDPOINTS, SOURCE, rows};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{
    CellKind, FactColumn, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use std::sync::Arc;

/// The nearest-ancestor questions asked of a graph's derived forest.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Over the edges 1 -> 2 -> 3 and 8 -> 2 with the identifiers {3}: the same domain twice from
    /// the root 1, then a computed domain {2}.
    MemoByDomain,
    /// Over the edges 1 -> 2 -> 3 with the identifiers {2}: from each root in turn.
    FromRoots([i64; 2]),
}

/// What the memoised questions observed.
#[derive(Debug, PartialEq)]
struct Memo {
    one_shared_domain: bool,
    domains: usize,
    first: Option<CellValue>,
    memo_after_first: usize,
    again_same: bool,
    forest_hits: u64,
    computed: Option<CellValue>,
    memo_after_computed: usize,
    memo_within_limit: bool,
    bytes_within_limit: bool,
}

/// What the questions answered.
#[derive(Debug, PartialEq)]
enum Answered {
    Memo(Memo),
    Nearest([Option<CellValue>; 2]),
}

type Input = Asked;
type Value = Answered;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nearest_memo_is_shared_only_for_exact_domain_and_generation"),
        word: Word::Succeeds,
        input: || Asked::MemoByDomain,
        expect: Expect::Value(Answered::Memo(Memo {
            one_shared_domain: true,
            domains: 1,
            first: Some(CellValue::Number(Integer(3))),
            memo_after_first: 1,
            again_same: true,
            forest_hits: 1,
            computed: Some(CellValue::Number(Integer(2))),
            memo_after_computed: 1,
            memo_within_limit: true,
            bytes_within_limit: true,
        })),
    },
    Case {
        name: CaseName("nearest_handles_absent_parent_and_normal_parent_through_derived_forest"),
        word: Word::Succeeds,
        input: || Asked::FromRoots([3, 1]),
        expect: Expect::Value(Answered::Nearest([
            None,
            Some(CellValue::Number(Integer(2))),
        ])),
    },
];

/// Acquires the graph and its identifier domain and asks the nearest identified ancestor.
fn subject(input: Input) -> Result<Value, Failure> {
    let (edges, members): ([FactColumn; 2], [FactColumn; 2]) = match input {
        Asked::MemoByDomain => (
            [FactColumn(vec![1, 2, 8]), FactColumn(vec![2, 3, 2])],
            [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])],
        ),
        Asked::FromRoots(_) => (
            [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])],
            [FactColumn(vec![2]), FactColumn(vec![0])],
        ),
    };
    let edge_rows = rows(&edges);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &edge_rows, ENDPOINTS, &[])
        .map_err(|error| format!("{error:?}"))?
        .ok_or("no graph")?;
    let member_rows = rows(&members);
    let domain = |rows: &Rows<'_>| {
        fixture
            .domain(
                &graph,
                PlanInput::Relation(Relation::Identifier),
                rows,
                ColumnIndex(0),
            )
            .map_err(|error| format!("{error:?}"))?
            .ok_or_else(|| "no domain".to_owned())
    };
    let membership = domain(&member_rows)?;
    let vertex = |value: i64| {
        graph
            .graph()
            .vertex(
                ryo_derived::domain::vertices::types::PartitionId(0),
                CellValue::Number(Integer(value)),
            )
            .map(|found| found.0)
            .ok_or_else(|| format!("no vertex {value}"))
    };
    let cell = |at: Option<u32>| {
        at.map(|at| graph.graph().vertices()[usize::try_from(at).expect("a vertex id fits in usize")])
    };
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    match input {
        Asked::FromRoots(roots) => {
            let mut answers = [None; 2];
            for (answer, root) in answers.iter_mut().zip(roots) {
                let at = fixture
                    .nearest(
                        &graph,
                        &membership,
                        vertex(root)?,
                        Orientation::Forward,
                        &mut local,
                        &mut walk,
                    )
                    .map_err(|error| format!("{error:?}"))?;
                *answer = cell(at);
            }
            Ok(Answered::Nearest(answers))
        }
        Asked::MemoByDomain => {
            let again = domain(&member_rows)?;
            let one_shared_domain = Arc::ptr_eq(membership.raw_domain(), again.raw_domain());
            let domains = fixture.snapshot().domains;
            let root = vertex(1)?;
            let first = fixture
                .nearest(
                    &graph,
                    &membership,
                    root,
                    Orientation::Forward,
                    &mut local,
                    &mut walk,
                )
                .map_err(|error| format!("{error:?}"))?;
            let memo_after_first = fixture.snapshot().nearest;
            let again_same = fixture
                .nearest(
                    &graph,
                    &again,
                    root,
                    Orientation::Forward,
                    &mut local,
                    &mut walk,
                )
                .map_err(|error| format!("{error:?}"))?
                == first;
            let forest_hits = fixture.snapshot().counts.forest_hits;
            // A computed domain cannot borrow stable relation membership identity.
            let table = ryo_values::domain::rows::types::Table {
                kinds: vec![CellKind::Number],
                cells: vec![CellValue::Number(Integer(2))],
                count: ryo_values::domain::rows::types::RowCount(1),
            };
            let different = domain(&Rows::Computed(&table))?;
            local.clear();
            let computed = fixture
                .nearest(
                    &graph,
                    &different,
                    root,
                    Orientation::Forward,
                    &mut local,
                    &mut walk,
                )
                .map_err(|error| format!("{error:?}"))?;
            let snapshot = fixture.snapshot();
            Ok(Answered::Memo(Memo {
                one_shared_domain,
                domains,
                first: cell(first),
                memo_after_first,
                again_same,
                forest_hits,
                computed: cell(computed),
                memo_after_computed: snapshot.nearest,
                memo_within_limit: snapshot.memo_bytes
                    <= super::super::super::super::service::MEMO_LIMIT,
                bytes_within_limit: snapshot.bytes <= LIMIT,
            }))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
