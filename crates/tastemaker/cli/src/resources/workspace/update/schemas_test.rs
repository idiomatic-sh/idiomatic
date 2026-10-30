//! What the schemas of `update` accept and refuse, one row of `CASES` for each command line.

use super::super::command::UpdateArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};

/// The command line, after `update`.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
/// What the user is told to do next about the refusal.
type Failure = Next;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: UpdateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || vec!["crate-core", "--to", "0.12.0"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_version_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || vec!["--to", "twelve"],
        expect: Expect::Refusal(|next| {
            *next == Next::Fix("pass `--to major.minor.patch`".to_owned())
        }),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("update").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    super::request(parsed.arguments.entry, parsed.arguments.to, &edge)
        .map(|_| true)
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
