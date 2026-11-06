//! A closure reads the graph its stored relation holds, through views that keep its columns, and nowhere else.

use super::value::graph_source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the value.
#[derive(Clone, Copy)]
enum Scenario {
    /// A closure over a stored relation of one file's edges.
    StoredEdges,
    /// A closure over the syntax tree, which has its own lookups.
    SyntaxTree,
}

/// Whether the graph is read from the stored relation.
type Value = bool;
type Failure = std::convert::Infallible;
type Input = Scenario;

const CASES: &[Case<Request, Value, Failure>] = &[
    Case {
        name: CaseName("a_stored_relation_is_the_graph_a_closure_reads"),
        word: Word::Succeeds,
        input: || Scenario::StoredEdges,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_syntax_tree_is_not_pulled_into_a_closure"),
        word: Word::Succeeds,
        input: || Scenario::SyntaxTree,
        expect: Expect::Value(false),
    },
];

fn subject(scenario: Request) -> Result<Value, Failure> {
    let relation = match scenario {
        Scenario::StoredEdges => Relation::SemanticChild,
        Scenario::SyntaxTree => Relation::SyntaxTree,
    };
    let steps: Vec<PlanStep> = Vec::new();
    Ok(graph_source(
        Input::Relation(relation),
        ColumnIndex(0),
        ColumnIndex(1),
        &[],
        &steps,
    )
    .is_some())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
