//! What `relation list` promises its caller, one row of `CASES` for each case it has.

use super::command::{ListArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;

/// What a row gives the command: the command line, one argument at a time.
type Input = Vec<&'static str>;
/// How many lines the command prints when it succeeds, and the format its command line chose.
type Value = (usize, crate::output::Format);
/// How the command ends when it does not succeed.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: ListArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
}

/// The catalog includes source coordinates, lexical facts, optional parser capabilities and engine facts.
/// The rows of this command's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("prints_relations"),
    word: Word::Succeeds,
    input: || vec![],
    expect: Expect::Value((72, crate::output::Format::Terminal)),
}];

/// Runs `relation list` on one command line and returns what it printed or the outcome it ended with.
fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .map_err(|_refused| Outcome::Error)?;
    answer().map(|answer| (answer.to_string().lines().count(), parsed.output.format()))
}

/// The `succeeds` rows: the command line is accepted and the printed answer is the expected one.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
