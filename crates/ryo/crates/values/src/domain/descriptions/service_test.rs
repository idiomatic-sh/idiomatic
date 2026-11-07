//! The `descriptions` service's cases, on values alone.

use super::service::{describe, summarize};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many columns a relation is described with.
    Columns,
    /// Whether the summary and the description name the relation alike.
    SameName,
    /// Whether `defines`'s `kind` column carries the words `function` and `method`, and its `name` none.
    ClosedWords,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// A count, or whether the answer was right.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_is_described_with_all_its_columns"),
        word: Word::Succeeds,
        input: || Scenario::Columns,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("the_summary_and_the_description_name_the_relation_alike"),
        word: Word::Succeeds,
        input: || Scenario::SameName,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_column_of_a_closed_set_carries_its_words"),
        word: Word::Succeeds,
        input: || Scenario::ClosedWords,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = Relation::Function;
    Ok(match scenario {
        Scenario::Columns => describe(relation).columns.len(),
        Scenario::SameName => usize::from(summarize(relation).name == describe(relation).name),
        Scenario::ClosedWords => {
            let columns = describe(Relation::Defines).columns;
            let words = |name: &str| {
                columns
                    .iter()
                    .find(|column| &*column.name.0 == name)
                    .map(|column| {
                        column
                            .words
                            .iter()
                            .map(|word| word.0.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };
            let kinds = words("kind");
            usize::from(
                kinds.iter().any(|word| word == "function")
                    && kinds.iter().any(|word| word == "method")
                    && words("name").is_empty(),
            )
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
