//! What the schemas of this command accept and refuse, one row of `CASES` for each input.

use super::super::command::RunArguments;
use super::RunSchemaError;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};

/// What a row gives the schemas: a command line, as clap reads it, or the error a stream refuses every write of the JSON answer
/// with.
#[derive(Debug)]
enum Given {
    /// The command line, one argument at a time.
    Line(Vec<&'static str>),
    /// What the stream the JSON answer is written to refuses each write with: a reader that closed it, or a full disk.
    Stream(std::io::ErrorKind),
}

/// What the schemas answer.
#[derive(Debug, PartialEq, Eq)]
enum Answered {
    /// Of a command line: how many rows the request lets through, and how many of its rules are named by their place among them,
    /// from 1.
    Request(u64, usize),
    /// Of a stream: the run ended with no error.
    Delivered,
}

/// What a row gives the schemas.
type Input = Given;
/// What the schemas answer.
type Value = Answered;
type Failure = RunSchemaError;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// Where the command line looks for the project, which the command tree reads into the edge.
    #[arg(long)]
    root: Option<String>,
    #[command(flatten)]
    arguments: RunArguments,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || Given::Line(vec!["--text", "x"]),
        expect: Expect::Value(Answered::Request(20, 1)),
    },
    Case {
        name: CaseName("the_limit_is_the_one_the_line_gives"),
        word: Word::Succeeds,
        input: || Given::Line(vec!["--text", "x", "--limit", "5"]),
        expect: Expect::Value(Answered::Request(5, 1)),
    },
    Case {
        name: CaseName("each_text_is_a_rule_named_by_its_place"),
        word: Word::Succeeds,
        input: || Given::Line(vec!["--text", "x", "--text", "y"]),
        expect: Expect::Value(Answered::Request(20, 2)),
    },
    Case {
        name: CaseName("a_blank_value_is_refused"),
        word: Word::Refuses,
        input: || Given::Line(vec!["--text", " "]),
        expect: Expect::Error(RunSchemaError::BlankText),
    },
    Case {
        name: CaseName("a_blank_value_among_several_is_refused"),
        word: Word::Refuses,
        input: || Given::Line(vec!["--text", "x", "--text", " "]),
        expect: Expect::Error(RunSchemaError::BlankText),
    },
    Case {
        name: CaseName("a_closed_stream_ends_the_json_answer_as_closed"),
        word: Word::Succeeds,
        input: || Given::Stream(std::io::ErrorKind::BrokenPipe),
        expect: Expect::Value(Answered::Delivered),
    },
    Case {
        name: CaseName("fails_when_the_disk_fails_saying_why"),
        word: Word::Fails,
        input: || Given::Stream(std::io::ErrorKind::StorageFull),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, RunSchemaError::Output(reason) if !reason.is_empty()),
            )
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Given::Line(line) => requested(line),
        Given::Stream(kind) => json_ended_by(kind).map(|()| Answered::Delivered),
    }
}

/// Reads a command line into the request the schemas build of it.
fn requested(line: Vec<&'static str>) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .expect("the row's command line is well formed");
    super::request(
        parsed.root.unwrap_or_else(|| ".".to_owned()),
        &parsed.arguments.text,
        parsed.arguments.limit,
    )
    .map(|request| {
        let (limit, named) = (request.limit.0, request.queries.len());
        Answered::Request(limit, named)
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// A stream whose every write ends in `kind`: a reader that closed, or a disk that is full.
struct Refusing(std::io::ErrorKind);

impl std::io::Write for Refusing {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(self.0))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::from(self.0))
    }
}

/// How the run ends when its whole JSON answer is written onto a stream that refuses every write with `kind`: through the output
/// that holds the document, `unwritten` for the refusal, and `delivered` for the end of the command.
fn json_ended_by(kind: std::io::ErrorKind) -> Result<(), RunSchemaError> {
    let ended = crate::output::written(
        crate::output::Format::Json,
        Box::new(Refusing(kind)),
        |mut held| {
            std::io::Write::write_all(&mut held, b"{}").map_err(|error| super::unwritten(&error))
        },
        super::unwritten,
    )
    ;
    super::delivered(ended)
}
