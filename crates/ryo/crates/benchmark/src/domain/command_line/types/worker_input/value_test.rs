//! A worker's arguments name the request file after `--request-file`, or carry the request inline.
use super::super::BenchmarkArgument;
use super::value::WorkerInput;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

type Input = &'static [&'static str];
/// The request source and the arguments left, rendered as text.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the request file is read from the path after --request-file"),
        word: Word::Succeeds,
        input: || &["--request-file", "/tmp/request.json", "rest"],
        expect: Expect::Value("File(\"/tmp/request.json\"); left [\"rest\"]"),
    },
    Case {
        name: CaseName("any other first argument is the inline JSON request"),
        word: Word::Succeeds,
        input: || &["{\"root\":\"/tmp\"}"],
        expect: Expect::Value("Inline(InlineRequest(\"{\\\"root\\\":\\\"/tmp\\\"}\")); left []"),
    },
    Case {
        name: CaseName("a worker command line without a request is refused"),
        word: Word::Refuses,
        input: || &[],
        expect: Expect::Error("missing worker request"),
    },
    Case {
        name: CaseName("--request-file without its path is refused"),
        word: Word::Refuses,
        input: || &["--request-file"],
        expect: Expect::Error("missing worker request"),
    },
];

fn subject(arguments: Input) -> Result<Value, Failure> {
    let mut arguments = arguments
        .iter()
        .map(|argument| BenchmarkArgument((*argument).into()));
    let input = WorkerInput::read(&mut arguments).map_err(|error| match error {
        Error::Invalid(message) => &*Box::leak(message.into_boxed_str()),
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => "unexpected",
    })?;
    Ok(Box::leak(
        format!(
            "{input:?}; left {:?}",
            arguments.map(|argument| argument.0).collect::<Vec<_>>()
        )
        .into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
