//! What the schemas of this command accept and refuse, one row of `CASES` for each input.

use super::super::command::DescribeArguments;
use super::DescribeSchemaError;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The command line, as clap reads it.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
type Failure = DescribeSchemaError;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: DescribeArguments,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || vec!["x"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_blank_value_is_refused"),
        word: Word::Refuses,
        input: || vec![" "],
        expect: Expect::Error(DescribeSchemaError::BlankName),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .expect("the row's command line is well formed");
    super::request(parsed.arguments.name).map(|_| true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
