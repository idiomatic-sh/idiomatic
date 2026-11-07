//! What `create` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{CreateArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{dry_run, edge, workspace};

/// The command line, after `create`.
type Input = Vec<&'static str>;
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: CreateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_would_be_written_at_its_first_version"),
        word: Word::Succeeds,
        input: || vec!["rule/no-tsv", "--set", "extension=rs", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_with_no_extension_named_is_refused_never_guessed"),
        word: Word::Refuses,
        input: || vec!["rule/no-tsv", "-n"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_value_no_skeleton_reads_is_refused"),
        word: Word::Refuses,
        input: || vec!["rule/no-tsv", "--set", "language=txt", "-n"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_name_that_is_not_kebab_case_is_refused_as_an_error"),
        word: Word::Refuses,
        input: || vec!["rule/Bad_Name", "--set", "extension=rs", "-n"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_definition_without_its_kind_is_refused"),
        word: Word::Refuses,
        input: || vec!["no-tsv", "-n"],
        expect: Expect::error(Outcome::Error),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let directory = workspace();
    // `-n` is the command line's, taken before the command reaches its arguments.
    let dry = line.contains(&"-n");
    let line = line.into_iter().filter(|argument| *argument != "-n");
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("create").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = if dry {
        dry_run(edge(&directory))
    } else {
        edge(&directory)
    };
    let answer = answer(parsed.arguments, &edge)?;
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("rules/no-tsv/0.1.0/rule.toml"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
