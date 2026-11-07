//! What the schemas of this command accept and refuse, one row of `CASES` for each input.

use super::super::command::ValidateArguments;
use super::ValidateSchemaError;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_core::inbound::use_cases::validate::types::ValidateRequest;
use ryo_values::domain::plan_parts::types::QueryText;

/// The command line, as clap reads it.
type Input = Vec<&'static str>;
/// Whether the arguments became the request the SDK builds for the same text when its caller declares no relation: the text, and no
/// relation of the host, since the command line declares none.
type Value = bool;
type Failure = ValidateSchemaError;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ValidateArguments,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_the_request_the_sdk_builds_with_no_declaration"),
        word: Word::Succeeds,
        input: || vec!["--text", "x"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_blank_value_is_refused"),
        word: Word::Refuses,
        input: || vec!["--text", " "],
        expect: Expect::Error(ValidateSchemaError::BlankText),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .expect("the row's command line is well formed");
    let text = parsed.arguments.text;
    let expected = ValidateRequest {
        observations: Vec::new(),
        text: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
            text.as_str().into(),
        )),
        hosted: Vec::new(),
    };
    super::request(text).map(|request| request == expected)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
