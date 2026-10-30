//! What the schemas of `add` accept and refuse, one row of `CASES` for each command line.

use super::super::command::AddArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};

/// The command line, after `add`.
type Input = Vec<&'static str>;
/// Whether the arguments became a request.
type Value = bool;
/// What the user is told to do next about the refusal.
type Failure = Next;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: AddArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || vec!["crate", "--at", "crates/core"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_next_command_preserves_shell_arguments"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "src/owner's space"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_is_checked_next_not_scaffolded"),
        word: Word::Succeeds,
        input: || vec!["marker", "--at", "src"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_capture_is_left_for_the_instance_name"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "src/services/{name}"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_param_with_no_value_is_refused"),
        word: Word::Refuses,
        input: || vec!["crate", "--at", "crates/core", "--param", "package"],
        expect: Expect::Refusal(|next| {
            *next == Next::Fix("write each `--param` as `name=value`".to_owned())
        }),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("add").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo/work space".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    let expected = parsed.arguments.at.to_str().expect("text path").to_owned();
    let definition = parsed.arguments.definition.clone();
    let request = super::request(
        parsed.arguments.definition,
        parsed.arguments.at,
        parsed.arguments.params,
        &edge,
    )
    .map_err(|error| error.next())?;
    // `marker` is a rule here, anything else a spec.
    let kind = if definition == "marker" {
        tastemaker_core::domain::packages::types::DefinitionKind::Rule
    } else {
        tastemaker_core::domain::packages::types::DefinitionKind::Spec
    };
    let report = tastemaker_core::inbound::use_cases::add::types::AddReport {
        file: tastemaker_core::domain::paths::types::RepoPath("tastemaker.toml".into()),
        entry: tastemaker_core::inbound::use_cases::add::types::ShownEntry(
            format!("p/{definition}@0.1.0").into(),
        ),
        reference: tastemaker_core::domain::packages::types::DefinitionName(
            definition.clone().into(),
        ),
        kind,
        novelty: tastemaker_core::inbound::use_cases::add::types::Novelty::New,
    };
    let answer = super::answer(&report, false, &request.at, None).map_err(|error| error.next())?;
    // The command runs where `add` ran: it names no `--root`, so no machine path.
    let next: Vec<String> = if definition == "marker" {
        vec!["tm".into(), "check".into()]
    } else {
        vec![
            "tm".into(),
            "scaffold".into(),
            definition,
            "--at".into(),
            expected.replace("{name}", "<name>"),
        ]
    };
    Ok(*request.at.0 == expected && shlex::split(&answer.next) == Some(next))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
