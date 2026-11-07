//! What `explain` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{ExplainArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{edge, workspace};

/// The command line, after `explain`.
type Input = Vec<&'static str>;
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ExplainArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_says_the_entry_that_places_it"),
        word: Word::Succeeds,
        input: || vec!["src/a/errors.toml"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_path_says_the_scaffold_command_of_the_entry_that_places_it"),
        word: Word::Succeeds,
        input: || vec!["src/a/errors.toml"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_name_that_is_nothing_is_refused"),
        word: Word::Refuses,
        input: || vec!["nothing-of-that-name"],
        expect: Expect::error(Outcome::Error),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let directory = workspace();
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("explain").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = edge(&directory);
    let answer = answer(parsed.arguments, &edge)?;
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("placed by `p/part/errors`")
        && said.contains("scaffolded by `tm scaffold p/part/errors --at src/a`")
        && said.contains("scaffolded by `tm scaffold p/part --at src/a`"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
