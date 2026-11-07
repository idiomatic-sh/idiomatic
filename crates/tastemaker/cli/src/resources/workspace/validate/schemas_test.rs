//! What the schemas of `validate` accept and refuse, one row of `CASES` for each command line.

use super::super::command::ValidateArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The command line, after `validate`.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
type Failure = ();

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ValidateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("arguments_become_a_request"),
    word: Word::Succeeds,
    input: || vec!["file-length"],
    expect: Expect::Value(true),
}];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("validate").chain(line))
        .map_err(|_error| ())?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    Ok(super::request(parsed.arguments.only, &edge).only.len() == 1)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
