//! What the schemas of `scaffold` accept and refuse, one row of `CASES` for each command line.

use super::super::command::ScaffoldArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};

/// The command line, after `scaffold`.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
/// What the user is told to do next about the refusal.
type Failure = Next;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ScaffoldArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || vec!["part", "--set", "domain=greeting"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_current_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "."],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_hidden_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", ".config/x"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_parent_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "../outside"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_relative_hidden_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "./.config/x"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_absolute_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "/outside"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("target_root_is_preserved"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "/"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_setting_with_no_value_is_refused"),
        word: Word::Refuses,
        input: || vec!["part", "--set", "domain"],
        expect: Expect::Refusal(|next| {
            *next == Next::Fix("write each `--set` as `name=value`".to_owned())
        }),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("scaffold").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    let target = parsed.arguments.at.to_str().expect("text path").to_owned();
    let settings = parsed.arguments.set.len();
    super::request(
        parsed.arguments.entry,
        parsed.arguments.at,
        parsed.arguments.set,
        &edge,
    )
    .map(|request| request.set.len() == settings && *request.at.0 == target)
    .map_err(|error| error.next())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
