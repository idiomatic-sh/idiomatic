//! What `relation describe` promises its caller, one row of `CASES` for each case it has.

use super::command::{DescribeArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;

/// What a row gives the command: the command line, one argument at a time.
type Input = Vec<&'static str>;
/// How many lines the command prints when it succeeds.
type Value = usize;
/// How the command ends when it does not succeed.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: DescribeArguments,
}

/// The rows of this command's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prints_one_relation_with_its_columns_and_the_languages_that_read_it"),
        word: Word::Succeeds,
        input: || vec!["function"],
        // The name line, four columns and one line for each linked language: rust always, the others when they are built.
        expect: Expect::Value(
            6 + (if cfg!(feature = "css") { 1 } else { 0 })
                + (if cfg!(feature = "go") { 1 } else { 0 })
                + (if cfg!(feature = "html") { 1 } else { 0 })
                + (if cfg!(feature = "javascript") { 1 } else { 0 })
                + (if cfg!(feature = "json") { 1 } else { 0 })
                + (if cfg!(feature = "markdown") { 1 } else { 0 })
                + (if cfg!(feature = "python") { 1 } else { 0 })
                + (if cfg!(feature = "toml") { 1 } else { 0 })
                + (if cfg!(feature = "typescript") { 1 } else { 0 }),
        ),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || vec!["nothing"],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("a_blank_name_is_refused"),
        word: Word::Refuses,
        input: || vec![" "],
        expect: Expect::Error(Outcome::Failure),
    },
];

/// Runs `relation describe` on one command line and returns what it printed or the outcome it ended with.
fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .map_err(|_refused| Outcome::Error)?;
    answer(parsed.arguments).map(|answer| answer.to_string().lines().count())
}

/// The `succeeds` rows: the command line is accepted and the printed answer is the expected one.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The `refuses` rows: the command line is refused with the expected outcome.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
