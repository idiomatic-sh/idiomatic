//! A closure reads the graph its stored relation holds, through views that keep its columns, and nowhere else.

use super::value::graph_source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use std::borrow::Cow;

/// What a row asks of the value.
#[derive(Clone, Copy)]
enum Scenario {
    /// A closure over a stored relation of one file's edges.
    StoredEdges,
    /// A closure over the syntax tree, which has its own lookups.
    SyntaxTree,
    /// A closure over so many filters that keep every row of a stored relation, each of the one before, and over twice as many.
    Chain(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// Whether the graph is read from the stored relation.
    Pulled(bool),
    /// Where a chain of views was pulled to at a length and at twice it: the walk follows the one input of each view, a step at a
    /// time and no more steps than the plan has, so it reaches the relation in as many steps as the chain is long.
    Counted(Cow<'static, str>),
}

type Value = Answer;
type Failure = std::convert::Infallible;
/// What a row asks of the value.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_stored_relation_is_the_graph_a_closure_reads"),
        word: Word::Succeeds,
        input: || Scenario::StoredEdges,
        expect: Expect::Value(Answer::Pulled(true)),
    },
    Case {
        name: CaseName("the_syntax_tree_is_not_pulled_into_a_closure"),
        word: Word::Succeeds,
        input: || Scenario::SyntaxTree,
        expect: Expect::Value(Answer::Pulled(false)),
    },
    Case {
        name: CaseName("a_chain_of_views_is_pulled_through_once_a_step"),
        word: Word::Succeeds,
        input: || Scenario::Chain(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 views: pulled to Some(Relation(SemanticChild)); 32 views: pulled to Some(Relation(SemanticChild))",
        ))),
    },
];

/// Where a closure over `length` filters of the stored edges that keep every row, each of the one before, reads its graph from.
fn pulled_through(length: usize) -> String {
    let mut steps = Vec::new();
    let mut before = Reads::Relation(Relation::SemanticChild);
    for _ in 0..length {
        steps.push(PlanStep {
            written: Default::default(),
            operation: Operation::Select {
                from: before,
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        });
        before = Reads::Step(StepId(
            u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
        ));
    }
    let pulled =
        graph_source(before, ColumnIndex(0), ColumnIndex(1), &[], &steps).map(|graph| graph.input);
    format!("{length} views: pulled to {pulled:?}")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = match scenario {
        Scenario::StoredEdges => Relation::SemanticChild,
        Scenario::SyntaxTree => Relation::SyntaxTree,
        Scenario::Chain(length) => {
            return Ok(Answer::Counted(Cow::Owned(format!(
                "{}; {}",
                pulled_through(length),
                pulled_through(length.saturating_mul(2))
            ))));
        }
    };
    let steps: Vec<PlanStep> = Vec::new();
    Ok(Answer::Pulled(
        graph_source(
            Reads::Relation(relation),
            ColumnIndex(0),
            ColumnIndex(1),
            &[],
            &steps,
        )
        .is_some(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
