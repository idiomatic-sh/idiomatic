//! What `list` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{ListArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{edge, workspace};

/// The command line, after `list`.
type Input = Vec<&'static str>;
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ListArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_definition_is_listed"),
        word: Word::Succeeds,
        input: Vec::new,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_term_after_the_kind_filters_the_list"),
        word: Word::Succeeds,
        input: || vec!["rules", "marker"],
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_word_nothing_holds_is_an_error"),
        word: Word::Refuses,
        input: || vec!["bogus"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_kind_that_is_none_of_the_four_is_refused"),
        word: Word::Refuses,
        input: || vec!["modules", "marker"],
        expect: Expect::error(Outcome::Failure),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let directory = workspace();
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("list").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = edge(&directory);
    let answer = answer(parsed.arguments, &edge)?;
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    // The package and the name are fields of their own, and a field with nothing to say is left out.
    Ok(said.contains(r#""package":"p","name":"marker""#)
        && said.contains(r#""package":"p","name":"part""#)
        && !said.contains(r#":"""#))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
