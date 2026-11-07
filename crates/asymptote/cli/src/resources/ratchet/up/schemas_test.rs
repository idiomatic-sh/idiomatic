//! What the schemas of `up` accept and refuse: the reason, and each `--at` as `check` prints a violation.

use clap::error::ErrorKind;

use super::super::command::UpArguments;
use super::UpSchemaError;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The command line, after `up`.
type Input = Vec<&'static str>;
/// The locations the request selects, `path:line`, joined.
type Value = std::borrow::Cow<'static, str>;
/// Who refused the line: the parser, with the kind of its error, or the schemas.
type Failure = Refused;

/// Who refused a command line.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Refused {
    /// The command line parser, with the kind of error it gave.
    Line(ErrorKind),
    /// The schemas, with their error.
    Schema(UpSchemaError),
}

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: UpArguments,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_at_as_check_prints_it_drops_its_column"),
        word: Word::Succeeds,
        input: || {
            vec![
                "--reason",
                "why",
                "--at",
                "src/a.rs:12:5",
                "--at",
                "./b.rs:3",
            ]
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("src/a.rs:12 b.rs:3")),
    },
    Case {
        name: CaseName("selectors_without_an_at_select_no_location"),
        word: Word::Succeeds,
        input: || {
            vec![
                "--reason", "why", "--rule", "r", "--path", "src/**", "--tool", "lint",
            ]
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("up_takes_no_tolerance_the_tool_s_spread_gives_it"),
        word: Word::Refuses,
        input: || vec!["--reason", "why", "--tolerance-percent", "12.5"],
        expect: Expect::error(Refused::Line(ErrorKind::UnknownArgument)),
    },
    Case {
        name: CaseName("an_at_without_a_line_is_refused"),
        word: Word::Refuses,
        input: || vec!["--reason", "why", "--at", "src/a.rs"],
        expect: Expect::Refusal(|refused| matches!(refused, Refused::Schema(UpSchemaError::At(_)))),
    },
    Case {
        name: CaseName("an_at_with_line_zero_is_refused"),
        word: Word::Refuses,
        input: || vec!["--reason", "why", "--at", "src/a.rs:0"],
        expect: Expect::Refusal(|refused| matches!(refused, Refused::Schema(UpSchemaError::At(_)))),
    },
    Case {
        name: CaseName("a_blank_reason_is_refused"),
        word: Word::Refuses,
        input: || vec!["--reason", " "],
        expect: Expect::error(Refused::Schema(UpSchemaError::BlankReason)),
    },
    Case {
        name: CaseName("no_reason_is_refused"),
        word: Word::Refuses,
        input: Vec::new,
        expect: Expect::error(Refused::Line(ErrorKind::MissingRequiredArgument)),
    },
];

/// The places the request of the row's command line selects, `path:line`, joined.
fn told(line: Input) -> Result<String, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("up").chain(line))
        .map_err(|error| Refused::Line(error.kind()))?;
    let root = asymptote_core::domain::manifest::types::ProductRoot("/repo".into());
    let edge = crate::edge::Edge {
        here: root.clone(),
        start: root,
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build("test".into()),
        format: crate::output::Format::Terminal,
        writing: if parsed.dry_run.dry_run() {
            asymptote_core::domain::ratchet::types::Writing::DryRun
        } else {
            asymptote_core::domain::ratchet::types::Writing::Write
        },
    };
    let request = super::request(
        parsed.arguments.reason,
        parsed.arguments.rule,
        parsed.arguments.path,
        parsed.arguments.tool,
        parsed.arguments.at,
        &edge,
    )
    .map_err(Refused::Schema)?;
    let places = request
        .selectors
        .at
        .iter()
        .map(|at| format!("{}:{}", at.path.0, at.line.0))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(places)
}

/// Answers the row's command line with the places it selects.
fn subject(input: Input) -> Result<Value, Failure> {
    told(input).map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
