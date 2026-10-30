//! What the schemas of `create` accept and refuse, one row of `CASES` for each command line.

use super::super::command::CreateArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};
use tastemaker_core::domain::packages::types::{DefinitionKind, DefinitionName};
use tastemaker_core::inbound::use_cases::create::types::CreateReport;

/// What a row asks of `create`.
#[derive(Clone, Debug)]
enum Asked {
    /// The command line, after `create`.
    Line(Vec<&'static str>),
    /// The answer to a created definition of this kind, and the part of it the answer must name.
    Answer(DefinitionKind, &'static str),
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
    arguments: CreateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("arguments_become_a_request"),
        word: Word::Succeeds,
        input: || Asked::Line(vec!["spec/hex"]),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_created_names_its_cases_query_and_validate"),
        word: Word::Succeeds,
        input: || Asked::Answer(DefinitionKind::Rule, "query"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_spec_created_names_its_entries_and_validate"),
        word: Word::Succeeds,
        input: || Asked::Answer(DefinitionKind::Spec, "entries"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_template_created_names_its_content_and_validate"),
        word: Word::Succeeds,
        input: || Asked::Answer(DefinitionKind::Template, "content"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_kind_that_is_none_of_the_three_is_refused"),
        word: Word::Refuses,
        input: || Asked::Line(vec!["module/x"]),
        expect: Expect::Refusal(|next| {
            *next == Next::Fix("write `rule/<name>`, `spec/<name>` or `template/<name>`".to_owned())
        }),
    },
    Case {
        name: CaseName("an_extension_no_linked_language_reads_is_refused_naming_extension"),
        word: Word::Refuses,
        input: || Asked::Line(vec!["rule/no-tsv", "--set", "extension=txt"]),
        expect: Expect::Refusal(|next| {
            *next
                == Next::Fix(
                    "`--set extension=txt`: no linked language reads `.txt` files, so the scenarios would find nothing (`tm -V` names the languages linked); name an extension a linked language reads with `--set extension=<ext>`, as `--set extension=rs`".to_owned(),
                )
        }),
    },
    Case {
        name: CaseName("a_name_that_is_not_kebab_case_is_refused"),
        word: Word::Refuses,
        input: || Asked::Line(vec!["rule/Bad_Name"]),
        expect: Expect::Refusal(|next| {
            *next == Next::Fix("name it in kebab-case, as `rule/no-tsv`".to_owned())
        }),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let line = match asked {
        Asked::Line(line) => line,
        Asked::Answer(kind, part) => return Ok(next_line((kind, part)).is_empty()),
    };
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("create").chain(line))
        .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    super::request(
        parsed.arguments.definition,
        parsed.arguments.package,
        parsed.arguments.set,
        &edge,
        &crate::languages::linked(),
    )
    .map(|_| true)
    .map_err(|error| match (&error, error.next()) {
        // A refused extension is told with the message that names the parameter, then what to do.
        (super::CreateSchemaError::Extension(_), Next::Fix(fix)) => {
            Next::Fix(format!("{error}; {fix}"))
        }
        (_, next) => next,
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

/// What the answer says when it lacks a part it must name, or names a command the command line has not; empty when not.
fn next_line((kind, part): (DefinitionKind, &'static str)) -> String {
    let word = tastemaker_core::domain::packages::service::kind_word(kind).0;
    let report = CreateReport {
        package: tastemaker_core::domain::files::types::MachinePath("/repo/p".into()),
        file: tastemaker_core::domain::paths::types::RepoPath(
            format!("{word}s/x/0.1.0/{word}.toml").into(),
        ),
        kind,
        name: DefinitionName("x".into()),
    };
    let start = tastemaker_core::domain::files::types::MachinePath("/repo".into());
    let said = super::answer(&report, false, &start).to_string();
    let tree = crate::resources::command();
    let unknown = said
        .split("`tm ")
        .skip(1)
        .filter_map(|rest| rest.split([' ', '`']).next())
        .any(|command| tree.find_subcommand(command).is_none());
    // The file is named from where `tm` runs, never as a machine path.
    let relative = said.starts_with(&format!("wrote p/{word}s/x/0.1.0/{word}.toml\n"))
        && !said.contains("/repo");
    let whole = relative
        && said.contains(part)
        && said.contains(&format!("`tm validate {word}/x`"))
        && said.contains(&format!("`tm explain {word}`"))
        && !unknown;
    let quiet = !super::answer(&report, true, &start)
        .to_string()
        .contains("next:");
    if whole && quiet { String::new() } else { said }
}
