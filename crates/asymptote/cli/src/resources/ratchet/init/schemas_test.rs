//! What the schemas of `init` make of its switches, and the lines its answer prints.

use asymptote_core::domain::manifest::types::{HookName, ProductRoot};
use asymptote_core::domain::ratchet::types::Writing;
use asymptote_core::inbound::use_cases::init::types::{Configuration, InitReport};

use clap::error::ErrorKind;

use super::super::command::InitArguments;
use super::{answer, request};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What is asked.
#[derive(Clone, Debug)]
enum Question {
    /// The request these switches become.
    Switches(Vec<&'static str>),
    /// The lines a report prints.
    Report(
        Configuration,
        Writing,
        &'static [&'static str],
        &'static [&'static str],
    ),
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
/// The kind of error the command line parser refused the line with.
type Failure = ErrorKind;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: InitArguments,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_switch_writes"),
        word: Word::Succeeds,
        input: || Question::Switches(Vec::new()),
        expect: Expect::Value(std::borrow::Cow::Borrowed("Write")),
    },
    Case {
        name: CaseName("dry_run_writes_nothing"),
        word: Word::Succeeds,
        input: || Question::Switches(vec!["--dry-run"]),
        expect: Expect::Value(std::borrow::Cow::Borrowed("DryRun")),
    },
    Case {
        name: CaseName("a_hook_no_longer_listed_is_said_removed"),
        word: Word::Succeeds,
        input: || {
            Question::Report(
                Configuration::Kept,
                Writing::Write,
                &["pre-push"],
                &["pre-commit"],
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "kept asymptote.toml: it is there, and is left as it is\nwrote the pre-push hook\nremoved the pre-commit hook: it is no longer in [hooks]\nnext: asymptote check                      run the tools against the lock\nnext: asymptote up --reason <why>          accept what they find now, once\n",
        )),
    },
    Case {
        name: CaseName("a_dry_run_report_would_remove_and_gives_no_next_steps"),
        word: Word::Succeeds,
        input: || {
            Question::Report(
                Configuration::Created,
                Writing::DryRun,
                &["pre-commit"],
                &["pre-push"],
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "would write asymptote.toml and an empty lock\nwould write the pre-commit hook\nwould remove the pre-push hook: it is no longer in [hooks]\n",
        )),
    },
    Case {
        name: CaseName("a_switch_init_does_not_take_is_refused"),
        word: Word::Refuses,
        input: || Question::Switches(vec!["--groups", "lint"]),
        expect: Expect::error(ErrorKind::UnknownArgument),
    },
];

/// The hook names a row writes as text.
fn names(hooks: &[&str]) -> Vec<HookName> {
    hooks.iter().map(|hook| HookName((*hook).into())).collect()
}

/// Answers the row's question: the writing its switches ask for, or the lines a report prints.
fn subject(question: Input) -> Result<Value, Failure> {
    match question {
        Question::Switches(line) => {
            let parsed =
                <Line as clap::Parser>::try_parse_from(std::iter::once("init").chain(line))
                    .map_err(|error| error.kind())?;
            let edge = crate::edge::Edge {
                start: ProductRoot("/repo".into()),
                here: ProductRoot("/repo".into()),
                store: std::path::PathBuf::from(
                    asymptote_core::domain::operations::constants::CACHE,
                ),
                build: asymptote_core::outbound::check_store::types::Build("test".into()),
                format: crate::output::Format::Terminal,
                writing: if parsed.dry_run.dry_run() {
                    Writing::DryRun
                } else {
                    Writing::Write
                },
            };
            Ok(std::borrow::Cow::Owned(format!(
                "{:?}",
                request(&edge).writing
            )))
        }
        Question::Report(configuration, writing, installed, removed) => {
            let printed = answer(&InitReport {
                root: ProductRoot("/repo".into()),
                configuration,
                installed: names(installed),
                removed: names(removed),
                writing,
            });
            // `--format json` prints the bytes serde_json writes of the answer, on one line.
            assert_eq!(
                encoded(&printed, crate::output::Spacing::Compact),
                serde_json::to_vec(&printed).unwrap_or_default()
            );
            assert_eq!(
                encoded(&printed, crate::output::Spacing::Indented),
                serde_json::to_vec_pretty(&printed).unwrap_or_default()
            );
            Ok(std::borrow::Cow::Owned(printed.to_string()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// What `answer` writes as JSON, spaced as `spacing` says.
fn encoded(answer: &impl crate::output::Encoded, spacing: crate::output::Spacing) -> Vec<u8> {
    let mut written = Vec::new();
    answer.json(&mut written, spacing).unwrap_or_default();
    written
}
