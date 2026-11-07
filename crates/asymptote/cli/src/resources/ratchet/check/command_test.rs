//! What `check` promises its caller, one row of `CASES` for each case it has, over a product on disk.

use super::command::{CheckArguments, answer};
use super::fixtures::{Members, chosen, edge, measured, product, workspace};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;

/// What the command runs over.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// A product whose one tool prints this log in place of its own, when one is given.
    Lint(Option<Log>),
    /// The root of a workspace of this shape.
    Workspace(Members),
    /// A product whose one tool, `bench`, is a perf tool with no record in the lock.
    Measured,
}

/// A SARIF log the product's one tool prints in place of its own.
#[derive(Clone, Copy, Debug)]
enum Log {
    /// A run whose invocation did not succeed.
    Unsuccessful,
    /// A finding whose rule is named by its index in the driver's rules.
    IndexedRule,
    /// A result whose kind is `pass`.
    Pass,
}

impl Log {
    /// The log's SARIF text.
    const fn text(self) -> &'static str {
        match self {
            Self::Unsuccessful => {
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"invocations":[{"executionSuccessful":false}],"results":[]}]}"#
            }
            Self::IndexedRule => {
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[{"id":"r"}]}},"results":[{"rule":{"id":"r","index":0},"message":{"text":"finding"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":1}}}]}]}]}"#
            }
            Self::Pass => {
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"results":[{"ruleId":"r","kind":"pass","level":"none","message":{"text":"ok"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"}}}]}]}]}"#
            }
        }
    }
}

/// The command line, after `check`, and what it runs over.
type Input = (Vec<&'static str>, Scene);
/// What the terminal answer says, and how the command ends.
type Value = (std::borrow::Cow<'static, str>, Outcome);
/// How the command ends when it cannot answer.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: CheckArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_new_violation_is_one_compiler_style_line_then_the_summary"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Lint(None)),
        expect: Expect::Value((std::borrow::Cow::Borrowed(""), Outcome::Failure)),
    },
    Case {
        name: CaseName("format_sarif_prints_the_products_log_its_new_finding_new"),
        word: Word::Succeeds,
        input: || (vec!["--format", "sarif"], Scene::Lint(None)),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed("a SARIF log, its finding new"),
            Outcome::Failure,
        )),
    },
    Case {
        name: CaseName("a_since_that_names_nothing_is_refused"),
        word: Word::Refuses,
        input: || (vec!["--since", ""], Scene::Lint(None)),
        expect: Expect::error(Outcome::Failure),
    },
    Case {
        name: CaseName("an_unsuccessful_sarif_invocation_makes_the_command_unknown"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Lint(Some(Log::Unsuccessful))),
        expect: Expect::Value((std::borrow::Cow::Borrowed(""), Outcome::Error)),
    },
    Case {
        name: CaseName("an_indexed_rule_finding_makes_the_command_fail"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Lint(Some(Log::IndexedRule))),
        expect: Expect::Value((std::borrow::Cow::Borrowed(""), Outcome::Failure)),
    },
    Case {
        name: CaseName("a_pass_result_does_not_make_the_command_fail"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Lint(Some(Log::Pass))),
        expect: Expect::Value((std::borrow::Cow::Borrowed(""), Outcome::Success)),
    },
    Case {
        name: CaseName("a_workspace_root_checks_each_member_and_prefixes_its_lines_with_the_path"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Workspace(Members::Two)),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed(
                "crates/a: src/a.rs:1:5: lint/r: a marker\ncrates/a: 1 new, 0 resolved\ncrates/b: 0 new, 0 resolved\n",
            ),
            Outcome::Failure,
        )),
    },
    Case {
        name: CaseName("a_workspace_roots_own_tools_run_first_under_the_path_dot"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Workspace(Members::TwoAndRoot)),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed(
                ".: 0 new, 0 resolved\ncrates/a: src/a.rs:1:5: lint/r: a marker\ncrates/a: 1 new, 0 resolved\ncrates/b: 0 new, 0 resolved\n",
            ),
            Outcome::Failure,
        )),
    },
    Case {
        name: CaseName("groups_apply_to_every_member"),
        word: Word::Succeeds,
        input: || (vec!["--groups", "lint"], Scene::Workspace(Members::Two)),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed(
                "crates/a: src/a.rs:1:5: lint/r: a marker\ncrates/a: 1 new, 0 resolved\ncrates/b: 0 new, 0 resolved\n",
            ),
            Outcome::Failure,
        )),
    },
    Case {
        name: CaseName("a_group_no_tool_of_the_workspace_declares_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                vec!["--groups", "lint,nosuch"],
                Scene::Workspace(Members::Two),
            )
        },
        expect: Expect::error(Outcome::Failure),
    },
    Case {
        name: CaseName("a_plain_check_refuses_a_hook_mapping_a_group_no_tool_declares"),
        word: Word::Refuses,
        input: || (Vec::new(), Scene::Workspace(Members::UndeclaredHook)),
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_median_with_no_record_is_unseeded_and_says_how_to_seed_it"),
        word: Word::Succeeds,
        input: || (Vec::new(), Scene::Measured),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed(
                "unseeded: check/wall-ms 100; seed with `asymptote up --reason <why> --tool bench`\n0 new, 0 resolved; perf: 0 regressed, 1 unseeded, 0 improved, 0 resolved, 0 held\n",
            ),
            Outcome::Failure,
        )),
    },
    Case {
        name: CaseName("a_tool_outside_the_groups_is_not_run_or_reported"),
        word: Word::Succeeds,
        input: || (vec!["--groups", "lint"], Scene::Measured),
        expect: Expect::Value((
            std::borrow::Cow::Borrowed("0 new, 0 resolved\n"),
            Outcome::Success,
        )),
    },
    Case {
        name: CaseName("a_hook_runs_the_groups_the_root_maps_it_to"),
        word: Word::Succeeds,
        input: || (vec!["--hook", "pre-commit"], Scene::Lint(None)),
        expect: Expect::Value((std::borrow::Cow::Borrowed(""), Outcome::Failure)),
    },
    Case {
        name: CaseName("a_hook_the_root_does_not_map_is_refused_naming_the_hooks_it_maps"),
        word: Word::Refuses,
        input: || (vec!["--hook", "pre-push"], Scene::Lint(None)),
        expect: Expect::error(Outcome::Failure),
    },
    Case {
        name: CaseName("a_hook_with_groups_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                vec!["--hook", "pre-commit", "--groups", "lint"],
                Scene::Lint(None),
            )
        },
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_member_glob_matching_a_directory_without_a_manifest_is_an_error"),
        word: Word::Refuses,
        input: || (Vec::new(), Scene::Workspace(Members::OneWithoutManifest)),
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_workspace_has_no_one_sarif_log"),
        word: Word::Refuses,
        input: || (vec!["--format", "sarif"], Scene::Workspace(Members::Two)),
        expect: Expect::error(Outcome::Failure),
    },
];

/// Parses the row's command line and runs `check` over its scene, answering what the terminal says and how it ends.
fn subject((line, scene): Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("check").chain(line))
        .map_err(|error| crate::resources::usage(&error))?;
    let directory = match scene {
        Scene::Lint(_) => product(),
        Scene::Workspace(members) => workspace(members),
        Scene::Measured => measured(),
    }
    .expect("the scene's product is written");
    if let Scene::Lint(Some(log)) = scene {
        std::fs::write(directory.path().join("lint.sarif"), log.text())
            .expect("the tool's log is written");
    }
    let edge = chosen(edge(&directory), parsed.output, parsed.dry_run);
    let (said, outcome) = answer(parsed.arguments, &edge, |_| {})?;
    let text = said.to_string();
    // A SARIF log is said by what it holds: the one new finding, judged new against the lock.
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if compact.starts_with('{') && compact.contains("\"baselineState\":\"new\"") {
        return Ok((
            std::borrow::Cow::Borrowed("a SARIF log, its finding new"),
            outcome,
        ));
    }
    let expected = "src/a.rs:1:5: lint/r: a marker\n1 new, 0 resolved\n";
    let hidden = match scene {
        Scene::Lint(log) => log.is_some() || text == expected,
        Scene::Workspace(_) | Scene::Measured => false,
    };
    Ok((
        if hidden {
            std::borrow::Cow::Borrowed("")
        } else {
            std::borrow::Cow::Owned(text)
        },
        outcome,
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
