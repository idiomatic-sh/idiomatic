//! The captures a tool's inputs write: what a segment of one matches, the text a match writes, the tool a match
//! makes, and the capture a command line writes that no input does.

use super::service::{bind, captured, instance, names, pieces, substitute, uncaptured};
use super::types::{CaptureName, CaptureValue, Captures, CommandKey, Written};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::manifest::types::{Argument, Argv, Fix, Group, Output, Pathspec, Seconds, Tool};
use crate::domain::violations::types::ToolId;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The captures a pattern writes, in order and each once.
    Names,
    /// A regular expression's quantifier and a shell's braces are text.
    NotNames,
    /// A segment's pieces.
    Pieces,
    /// A capture alone matches a whole entry name.
    Whole,
    /// A capture between text matches what lies between.
    Between,
    /// An entry the text around a capture does not match.
    Unmatched,
    /// A capture already bound matches only its value.
    Bound,
    /// The values written into text.
    Substituted,
    /// The captures of a tool's inputs, in order and each once.
    Captured,
    /// The tool one match makes.
    Instance,
    /// A capture `run` writes that no input does.
    Uncaptured,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_names_its_captures_in_order_each_once"),
        word: Word::Succeeds,
        input: || Question::Names,
        expect: Expect::Value(std::borrow::Cow::Borrowed("lang,suite")),
    },
    Case {
        name: CaseName("braces_around_no_name_are_text"),
        word: Word::Succeeds,
        input: || Question::NotNames,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_segment_is_its_text_and_its_captures"),
        word: Word::Succeeds,
        input: || Question::Pieces,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "[Text(Literal(\"test_\")), Capture(CaptureName(\"name\")), Text(Literal(\".rs\"))]",
        )),
    },
    Case {
        name: CaseName("a_capture_alone_takes_the_whole_entry_name"),
        word: Word::Succeeds,
        input: || Question::Whole,
        expect: Expect::Value(std::borrow::Cow::Borrowed("lang=python")),
    },
    Case {
        name: CaseName("a_capture_between_text_takes_what_lies_between"),
        word: Word::Succeeds,
        input: || Question::Between,
        expect: Expect::Value(std::borrow::Cow::Borrowed("name=a.b")),
    },
    Case {
        name: CaseName("an_entry_the_text_does_not_match_binds_nothing"),
        word: Word::Succeeds,
        input: || Question::Unmatched,
        expect: Expect::Value(std::borrow::Cow::Borrowed("none")),
    },
    Case {
        name: CaseName("a_bound_capture_matches_only_its_value"),
        word: Word::Succeeds,
        input: || Question::Bound,
        expect: Expect::Value(std::borrow::Cow::Borrowed("lang=python none")),
    },
    Case {
        name: CaseName("the_values_are_written_into_the_text_and_an_unbound_capture_stays"),
        word: Word::Succeeds,
        input: || Question::Substituted,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "package(=idiomatic-sh-ryo-python) {other}",
        )),
    },
    Case {
        name: CaseName("a_tools_inputs_capture_in_the_order_they_write_them"),
        word: Word::Succeeds,
        input: || Question::Captured,
        expect: Expect::Value(std::borrow::Cow::Borrowed("lang,suite")),
    },
    Case {
        name: CaseName("one_match_makes_a_tool_of_its_own_id_command_and_inputs"),
        word: Word::Succeeds,
        input: || Question::Instance,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "test-language/python: nextest -E package(=idiomatic-sh-ryo-python) [languages/python, crates]",
        )),
    },
    Case {
        name: CaseName("a_capture_run_writes_that_no_input_does_is_refused"),
        word: Word::Refuses,
        input: || Question::Uncaptured,
        expect: Expect::Refusal(|actual| {
            if actual == "Run lang" {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// Captures of `pairs`, each a name and its value.
fn captures(pairs: &[(&str, &str)]) -> Captures {
    Captures(
        pairs
            .iter()
            .map(|(name, value)| (CaptureName((*name).into()), CaptureValue((*value).into())))
            .collect(),
    )
}

/// Captures as the rows write them: `name=value`, joined by commas, or `none`.
fn said(found: Option<Captures>) -> String {
    found.map_or_else(
        || "none".to_owned(),
        |found| {
            found
                .0
                .iter()
                .map(|(name, value)| format!("{}={}", name.0, value.0))
                .collect::<Vec<_>>()
                .join(",")
        },
    )
}

/// Names as the rows write them, joined by commas.
fn listed(found: &[CaptureName]) -> String {
    found
        .iter()
        .map(|name| &*name.0)
        .collect::<Vec<_>>()
        .join(",")
}

/// The tool the language rows ask about: `test-language`, run by nextest over the package of each language
/// directory, and `run` as given.
fn language_tool(run: &[&str]) -> Tool {
    Tool {
        id: ToolId("test-language".into()),
        run: Argv(run.iter().map(|word| Argument((*word).into())).collect()),
        fix: Fix::Absent,
        output: Output::ExitStatus,
        group: Group("check".into()),
        timeout: Seconds(60),
        inputs: vec![
            Pathspec("languages/{lang}".into()),
            Pathspec("crates".into()),
        ],
    }
}

fn subject(question: Input) -> Result<Value, Failure> {
    let segment = |text: &str, entry: &str, bound: &Captures| {
        said(bind(&pieces(Written(text)), Written(entry), bound))
    };
    let none = Captures::default();
    Ok(std::borrow::Cow::Owned(match question {
        Question::Names => listed(&names(Written("{lang}/{suite}/{lang}.rs"))),
        Question::NotNames => listed(&names(Written("^(<{7}|[|]{7})$ {a,b} {} {1x} ${HOME}/x"))),
        Question::Pieces => format!("{:?}", pieces(Written("test_{name}.rs")).0),
        Question::Whole => segment("{lang}", "python", &none),
        Question::Between => segment("test_{name}.rs", "test_a.b.rs", &none),
        Question::Unmatched => segment("test_{name}.rs", "lib.rs", &none),
        Question::Bound => {
            let bound = captures(&[("lang", "python")]);
            format!(
                "{} {}",
                segment("{lang}", "python", &bound),
                segment("{lang}", "go", &bound)
            )
        }
        Question::Substituted => {
            substitute(
                Written("package(=idiomatic-sh-ryo-{lang}) {other}"),
                &captures(&[("lang", "python")]),
            )
            .0
        }
        Question::Captured => listed(&captured(&[
            Pathspec(".".into()),
            Pathspec("languages/{lang}/{suite}".into()),
            Pathspec("fixtures/{lang}".into()),
        ])),
        Question::Instance => {
            let tool = language_tool(&["nextest", "-E", "package(=idiomatic-sh-ryo-{lang})"]);
            let made = instance(
                &tool,
                &[CaptureName("lang".into())],
                &captures(&[("lang", "python")]),
            );
            let run: Vec<&str> = made.run.0.iter().map(|word| &*word.0).collect();
            let inputs: Vec<&str> = made.inputs.iter().map(|input| &*input.0).collect();
            format!("{}: {} [{}]", made.id.0, run.join(" "), inputs.join(", "))
        }
        Question::Uncaptured => {
            // Only the refusal is the answer: a tool whose captures are all named is a failure of the row.
            let tool = Tool {
                inputs: vec![Pathspec(".".into())],
                ..language_tool(&["nextest", "-p", "idiomatic-sh-ryo-{lang}"])
            };
            if let Some(found) = uncaptured(&tool) {
                let key = match found.key {
                    CommandKey::Run => "Run",
                    CommandKey::Fix => "Fix",
                };
                return Err(format!("{key} {}", found.name.0));
            }
            String::new()
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
