//! What `scaffold` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{ScaffoldArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{dry_run, edge, workspace};

/// The command line, after `scaffold`.
type Input = Vec<&'static str>;
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ScaffoldArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_missing_file_of_an_instance_would_be_written"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "src/b", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_template_named_as_explain_names_it_would_write_the_missing_file"),
        word: Word::Succeeds,
        input: || vec!["p/part/errors", "--at", "src/b", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_spec_named_as_explain_names_it_would_write_the_missing_file"),
        word: Word::Succeeds,
        input: || vec!["p/part", "--at", "src/b", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_spec_named_at_its_version_would_write_the_missing_file"),
        word: Word::Succeeds,
        input: || vec!["p/part@0.1.0", "--at", "src/b", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_qualified_name_where_no_entry_places_it_is_refused"),
        word: Word::Refuses,
        input: || vec!["p/part/errors", "--at", "elsewhere/deep", "-n"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_setting_with_no_value_is_refused"),
        word: Word::Refuses,
        input: || vec!["part", "--set", "domain"],
        expect: Expect::error(Outcome::Failure),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let directory = workspace();
    // `-n` is the command line's, taken before the command reaches its arguments.
    let dry = line.contains(&"-n");
    let line = line.into_iter().filter(|argument| *argument != "-n");
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("scaffold").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = if dry {
        dry_run(edge(&directory))
    } else {
        edge(&directory)
    };
    let answer = answer(parsed.arguments, &edge)?;
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("src/b/errors.toml"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
