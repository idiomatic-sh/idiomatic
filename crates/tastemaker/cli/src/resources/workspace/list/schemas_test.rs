//! What the schemas of `list` accept and refuse, one row of `CASES` for each command line.

use super::super::command::ListArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};

/// The command line, after `list`.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
/// What the user is told to do next about the refusal.
type Failure = Next;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ListArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || vec!["rules", "length"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("one_word_that_is_no_kind_is_the_term"),
        word: Word::Succeeds,
        input: || vec!["length"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_kind_that_is_none_of_the_four_before_a_term_is_refused"),
        word: Word::Refuses,
        input: || vec!["modules", "length"],
        expect: Expect::Refusal(|next| *next == Next::Run("tm list".to_owned())),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("list").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    super::request(parsed.arguments.kind, parsed.arguments.term, &edge)
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
