//! What the `explain` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::ExplainError;
use super::interface::ExplainUseCase as _;
use super::types::{Asked, ExplainRequest, NameAsked};
use super::use_case::ExplainUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::explanations::constants::SKIPPED_NOTE;
use crate::domain::files::types::{Declarer, DirectoryName, MachinePath, SkippedDirectory};
use crate::inbound::use_cases::check::fixtures::{Workspace, package, repository_of, selection};
use crate::outbound::analysis::fixtures::fake::AnalysisFake;

/// What is asked.
type Input = (Workspace, &'static str);
/// The first section's title.
type Value = String;
type Failure = ExplainError;

/// Asks for the workspace itself, not a name.
const WORKSPACE: &str = "(the workspace)";

/// Asks for the workspace itself, and looks for the note that a root or an include the workspace names is read inside a skipped directory.
const WORKSPACE_NOTE: &str = "(the workspace, its note on skipped directories)";

/// Asks for `src/a/errors.txt`, and looks for the scaffold command of the entry that places it.
const SCAFFOLDED: &str = "(src/a/errors.txt, its scaffold command)";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_says_the_scaffold_command_of_the_entry_that_places_it"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, SCAFFOLDED),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_says_the_entry_that_places_it"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "src/a/errors.txt"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_kind_and_name_says_what_the_definition_is"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "rule/marker"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_kind_and_the_name_qualified_by_its_package_says_what_the_definition_is"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "rule/p/marker"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_kind_and_the_title_an_answer_writes_says_the_definition_again"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "rule/p/marker@0.1.0"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_qualified_by_a_package_that_has_no_such_definition_is_refused"),
        word: Word::Refuses,
        input: || (Workspace::Placed, "rule/q/marker"),
        expect: Expect::Refusal(|actual| selection(matches!(actual, ExplainError::Unknown { .. }))),
    },
    Case {
        name: CaseName("a_rule_says_its_query"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "rule/marker"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_relation_says_its_columns"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "relation/file"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_operator_says_its_example"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "operator/select"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_grammar_is_said"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "ryoql"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_form_topic_is_said"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "rule"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("nothing_asked_names_every_topic_and_how_to_author"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_relation_ryo_has_not_is_refused"),
        word: Word::Refuses,
        input: || (Workspace::Placed, "relation/nothing"),
        expect: Expect::Refusal(|actual| selection(matches!(actual, ExplainError::Unknown { .. }))),
    },
    Case {
        name: CaseName("the_workspace_says_the_directories_no_walk_enters_and_who_declares_them"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, WORKSPACE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "the_skipped_directories_say_a_root_or_an_include_the_workspace_names_is_read",
        ),
        word: Word::Succeeds,
        input: || (Workspace::Placed, WORKSPACE_NOTE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_topic_is_said"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, "selectors"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_that_is_nothing_is_refused"),
        word: Word::Refuses,
        input: || (Workspace::Placed, "nothing-of-that-name"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ExplainError::Unknown {
                    name: "nothing-of-that-name".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_topic_is_said_when_the_workspace_cannot_be_read"),
        word: Word::Succeeds,
        input: || (Workspace::Broken, "selectors"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_relation_is_said_when_the_workspace_cannot_be_read"),
        word: Word::Succeeds,
        input: || (Workspace::Broken, "relation/file"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_explain"),
        word: Word::Fails,
        input: || (Workspace::Broken, "src/a/errors.txt"),
        expect: Expect::Refusal(|error| selection(matches!(error, ExplainError::Manifest { .. }))),
    },
];

/// A line the answer holds, for each name asked.
fn expected(asked: &str) -> &'static str {
    match asked {
        "src/a/errors.txt" => "- placed by `p/part/errors`",
        "rule/marker" | "rule/p/marker" | "rule/p/marker@0.1.0" => "```ryoql",
        "relation/file" => "- `path` string",
        "operator/select" => "Example",
        "ryoql" => "RyoQL",
        "rule" => "rule",
        "" => "  2. tm create rule/<name> --set extension=<ext>",
        WORKSPACE => "- `node_modules`: javascript, typescript",
        WORKSPACE_NOTE => SKIPPED_NOTE,
        SCAFFOLDED => "- scaffolded by `tm scaffold p/part/errors --at src/a`",
        _ => "selectors",
    }
}

fn subject((workspace, asked): Input) -> Result<Value, Failure> {
    let mut repository = repository_of(workspace);
    repository.skipped = vec![SkippedDirectory {
        name: DirectoryName("node_modules".into()),
        by: vec![Declarer("javascript".into()), Declarer("typescript".into())],
    }];
    let explain = ExplainUseCaseImpl::new(
        Arc::new(repository),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(package()),
        Arc::new(crate::outbound::matching::factory::globset()),
        Arc::new(AnalysisFake::default()),
    );
    let report = explain.explain(ExplainRequest {
        start: MachinePath("/fake".into()),
        asked: if asked.is_empty() || asked == WORKSPACE || asked == WORKSPACE_NOTE {
            Asked::Workspace
        } else {
            Asked::Named(NameAsked(
                if asked == SCAFFOLDED {
                    "src/a/errors.txt"
                } else {
                    asked
                }
                .into(),
            ))
        },
    })?;
    let said = report
        .sections
        .iter()
        .flat_map(|section| {
            std::iter::once(section.title.0.to_string())
                .chain(section.lines.iter().map(|line| line.0.to_string()))
        })
        .collect::<Vec<_>>();
    Ok(if said.iter().any(|line| line == expected(asked)) {
        String::new()
    } else {
        said.join(" | ")
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
