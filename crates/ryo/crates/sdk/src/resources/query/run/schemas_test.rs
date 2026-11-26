//! What the schemas of `query run` map: a refusal of a query is the query's own, naming it by its place only among several, and the
//! failures of the linter are the failures of the run.

use super::{among, error};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::resources::query::RunError;
use crate::resources::linter::LintError;

/// What a row gives the schemas.
#[derive(Clone, Debug)]
enum Given {
    /// A query's failure among `several` queries, as the one at the place `id`.
    Among(RunError, &'static str, bool),
    /// A failure of the linter, with whether the run had several queries.
    Linter(LintError, bool),
}

/// What the schemas answer: the failure the caller reads, as it says itself.
type Value = &'static str;
/// A row's input.
type Input = Given;
/// No row of these cases ends in an error.
type Failure = std::convert::Infallible;

fn query(reason: &str) -> RunError {
    RunError::Query { reason: reason.to_owned() }
}

fn rule(id: &str) -> crate::resources::linter::RuleId {
    crate::resources::linter::RuleId::try_from(id).expect("an id")
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_query_is_refused_as_its_own"),
        word: Word::Succeeds,
        input: || Given::Among(query("x"), "1", false),
        expect: Expect::Value("x"),
    },
    Case {
        name: CaseName("one_of_several_is_refused_naming_its_place"),
        word: Word::Succeeds,
        input: || Given::Among(query("x"), "2", true),
        expect: Expect::Value("rule `2`: x"),
    },
    Case {
        name: CaseName("a_failure_that_is_not_a_query_is_not_named_among_several"),
        word: Word::Succeeds,
        input: || Given::Among(RunError::Corpus { reason: "r".to_owned() }, "2", true),
        expect: Expect::Value("corpus: r"),
    },
    Case {
        name: CaseName("a_corpus_that_cannot_be_listed_is_a_corpus_failure"),
        word: Word::Succeeds,
        input: || Given::Linter(LintError::Io { what: crate::resources::linter::Port::Corpus, reason: "r".to_owned() }, false),
        expect: Expect::Value("corpus: r"),
    },
    Case {
        name: CaseName("a_repository_that_does_not_fit_says_its_size"),
        word: Word::Succeeds,
        input: || Given::Linter(LintError::TooLarge { size: 5, budget: 3 }, false),
        expect: Expect::Value("the repository is 5 bytes and the budget is 3"),
    },
    Case {
        name: CaseName("a_process_over_the_budget_names_the_queries_by_their_places"),
        word: Word::Succeeds,
        input: || Given::Linter(LintError::MemoryBudgetExceeded { used: 9, budget: 3, plans: vec![rule("1"), rule("2")] }, true),
        expect: Expect::Value("the run holds 9 bytes and the budget is 3, in the rules 1, 2"),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let failure = match input {
        Given::Among(failure, id, several) => among(failure, id, several),
        Given::Linter(failure, several) => error(failure, several),
    };
    Ok(Box::leak(failure.to_string().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
