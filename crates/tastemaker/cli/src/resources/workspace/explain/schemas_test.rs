//! What the schemas of `explain` accept and refuse, one row of `CASES` for each command line.

use super::super::command::ExplainArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};

/// What a row asks of `explain`.
#[derive(Clone, Debug)]
enum Asked {
    /// The command line, after `explain`.
    Line(Vec<&'static str>),
    /// What `explain --help` is checked for.
    Help(Help),
}

type Input = Asked;
/// Whether the arguments became a request.
type Value = bool;
/// What the user is told to do next about the refusal.
type Failure = Next;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ExplainArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || Asked::Line(vec!["selectors"]),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explain_help_names_only_what_exists"),
        word: Word::Succeeds,
        input: || Asked::Help(Help::NamesOnlyWhatExists),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explain_help_names_every_topic"),
        word: Word::Succeeds,
        input: || Asked::Help(Help::NamesEveryTopic),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("all_with_a_subject_is_refused"),
        word: Word::Refuses,
        input: || Asked::Line(vec!["selectors", "--all", "--output", "out"]),
        expect: Expect::Refusal(|next| {
            *next == Next::Run("tm explain --all --output <dir>".to_owned())
        }),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let line = match asked {
        Asked::Line(line) => line,
        Asked::Help(check) => return Ok(help_gaps(check).is_empty()),
    };
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("explain").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    super::request(
        parsed.arguments.subject,
        parsed.arguments.all,
        parsed.arguments.output_directory,
        &edge,
    )
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

/// Every command, flag, topic and kind the text names that the command line or `explain` has not.
fn unknown(text: &str) -> Vec<String> {
    use tastemaker_core::domain::topics::service::names;
    let tree = crate::resources::command();
    let kinds = ["rule", "spec", "template", "relation", "operator"];
    let mut unknown = Vec::new();
    for named in text.split("tm ").skip(1) {
        let words = named
            .split([',', '`', '\n', '('])
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>();
        let Some(command) = words.first().and_then(|word| tree.find_subcommand(word)) else {
            unknown.push(named.lines().next().unwrap_or_default().to_owned());
            continue;
        };
        for word in &words[1..] {
            let known = match word.strip_prefix("--") {
                Some(flag) => {
                    flag == "help"
                        || command
                            .get_arguments()
                            .any(|argument| argument.get_long() == Some(flag))
                }
                None if command.get_name() == "explain" => {
                    // A placeholder, a path on disk, a topic, or `<kind>/<name>`.
                    word.starts_with('<')
                        || word.contains('.')
                        || names().iter().any(|name| name.0 == *word)
                        || word
                            .split_once('/')
                            .is_some_and(|(kind, _)| kinds.contains(&kind))
                }
                None if command.get_name() == "list" => {
                    let documented = command
                        .get_arguments()
                        .filter_map(|argument| argument.get_help())
                        .map(ToString::to_string)
                        .collect::<String>();
                    documented.contains(&format!("`{word}`"))
                }
                None => true,
            };
            if !known {
                unknown.push(format!("{} {word}", command.get_name()));
            }
        }
    }
    unknown
}

/// What a row checks of `explain --help`.
#[derive(Clone, Copy, Debug)]
enum Help {
    /// Every command, flag, topic and kind it names exists.
    NamesOnlyWhatExists,
    /// It names every topic.
    NamesEveryTopic,
}

/// What the help lacks or names wrongly; empty when nothing.
fn help_gaps(check: Help) -> Vec<String> {
    let said = super::help();
    match check {
        Help::NamesOnlyWhatExists => unknown(&said),
        Help::NamesEveryTopic => tastemaker_core::domain::topics::service::names()
            .into_iter()
            .filter(|name| !said.contains(name.0))
            .map(|name| name.0.to_owned())
            .collect(),
    }
}
