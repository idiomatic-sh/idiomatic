//! The globset adapter of the matching port: the port's contract, then what is specific to it, and what the selectors and
//! a workspace's `rules-exclude` answer over the globs and expressions it compiles.

use std::sync::Arc;

use super::adapter::MatchingGlobsetAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::matching::fixtures::contract::check;
use crate::outbound::matching::interface::MatchingPort as _;
use tastemaker_values::domain::captures::types::{
    CaptureName, CaptureValue, Captures, Interpolations,
};
use tastemaker_values::domain::definitions::fixtures::bare;
use tastemaker_values::domain::paths::types::RepoPath;
use tastemaker_values::domain::selectors::service::{
    covers, covers_any, group, leading, matches, place, resolve,
};
use tastemaker_values::domain::selectors::types::{
    Binding, Coverage, Match, PathPattern, PathRest, PathTest, PatternFailure, RegexText,
    SelectionContext, Selector, Selectors, Spelling, VariableName,
};
use tastemaker_values::domain::template_placing::errors::TemplatePlacingError;
use tastemaker_values::domain::template_placing::service::exclusion;

/// A question asked of a pattern the adapter compiles, placed at a base.
#[derive(Clone, Copy, Debug)]
enum Question {
    Matches(&'static str),
    Covers(&'static str),
    Leading,
    Group(&'static str, &'static str),
    /// A list that still holds a variable, resolved.
    Variable,
}

/// What a row hands the adapter.
enum Row {
    /// A spelling, and the path whose captures it binds.
    Bind(Spelling, &'static str),
    /// A pattern placed at a base, and the question asked of it.
    Selector(&'static str, &'static str, Question),
    /// An expression, and the path asked whether it is covered.
    Regex(&'static str, &'static str),
    /// The files a workspace's `rules-exclude` of one selector covers.
    Exclude(Selector),
}

/// What the row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The values a spelling binds, empty when they are the ones expected.
    Bound(Vec<String>),
    /// The answer in words, empty when it is the one expected.
    Said(String),
    /// Whether the path is covered.
    Covered(bool),
}

/// Why the row is refused.
#[derive(Debug, PartialEq)]
enum Refused {
    /// The pattern does not compile, or misses.
    Pattern,
    /// The `rules-exclude` does not compile.
    Placing(TemplatePlacingError),
}

type Input = Row;
type Value = Answer;
type Failure = Refused;

fn binding(glob: &str, expression: &str, groups: &[&str]) -> Spelling {
    Spelling {
        glob: glob.into(),
        binder: Binding::Expression(expression.into()),
        groups: groups
            .iter()
            .map(|name| CaptureName((*name).into()))
            .collect(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_expression_binds_each_capture_in_order"),
        word: Word::Succeeds,
        input: || {
            Row::Bind(
                binding("src/*", "^src/([^/]+)$", &["domain"]),
                "src/greeting",
            )
        },
        expect: Expect::Value(Answer::Bound(Vec::new())),
    },
    Case {
        name: CaseName("a_glob_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Row::Bind(binding("src/[", "^src$", &[]), "src"),
        expect: Expect::error(Refused::Pattern),
    },
    Case {
        name: CaseName("an_expression_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Row::Bind(binding("src/*", "^(src$", &[]), "src"),
        expect: Expect::error(Refused::Pattern),
    },
    Case {
        name: CaseName("a_capture_binds_one_segment"),
        word: Word::Succeeds,
        input: || {
            Row::Selector(
                "crates",
                "{crate}/src",
                Question::Matches("crates/core/src"),
            )
        },
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_span_capture_binds_whole_segments"),
        word: Word::Succeeds,
        input: || Row::Selector("", "{*parent}/{member}", Question::Matches("a/b/c")),
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_pattern_covers_what_lies_beneath_its_match"),
        word: Word::Succeeds,
        input: || Row::Selector("", "crates/*", Question::Covers("crates/core/src/lib.txt")),
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_pattern_does_not_cover_a_sibling_with_its_prefix"),
        word: Word::Succeeds,
        input: || Row::Selector("", "crates/core", Question::Covers("crates/core2/lib.txt")),
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("the_names_before_the_last_capture_lead"),
        word: Word::Succeeds,
        input: || Row::Selector("", "crates/{folder}/{member}", Question::Leading),
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_group_is_the_directory_of_the_last_capture"),
        word: Word::Succeeds,
        input: || {
            Row::Selector(
                "",
                "crates/{folder}/{member}",
                Question::Group("folder", "ryo"),
            )
        },
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_name_bound_twice_with_two_values_misses"),
        word: Word::Refuses,
        input: || Row::Selector("", "{x}/{x}", Question::Matches("a/b")),
        expect: Expect::error(Refused::Pattern),
    },
    Case {
        name: CaseName("a_variable_left_in_a_list_is_refused_at_placement"),
        word: Word::Refuses,
        input: || Row::Selector("", "src", Question::Variable),
        expect: Expect::error(Refused::Pattern),
    },
    Case {
        name: CaseName("a_path_the_expression_matches_is_covered"),
        word: Word::Succeeds,
        input: || Row::Regex(r"^src/.*_test\.txt$", "crate/src/a_test.txt"),
        expect: Expect::Value(Answer::Covered(true)),
    },
    Case {
        name: CaseName("a_path_it_does_not_match_is_not"),
        word: Word::Succeeds,
        input: || Row::Regex(r"^src/.*_test\.txt$", "crate/src/a.txt"),
        expect: Expect::Value(Answer::Covered(false)),
    },
    Case {
        name: CaseName("a_regex_selector_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Row::Regex("(", "crate/src/a.txt"),
        expect: Expect::error(Refused::Pattern),
    },
    Case {
        name: CaseName("a_rules_exclude_covers_the_files_beneath_its_glob"),
        word: Word::Succeeds,
        input: || Row::Exclude(Selector::Glob(PathPattern("docs/**".into()))),
        expect: Expect::Value(Answer::Said(String::new())),
    },
    Case {
        name: CaseName("a_rules_exclude_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Row::Exclude(Selector::Regex(RegexText("(".into()))),
        expect: Expect::Refusal(|actual| {
            if actual
                == &Refused::Placing(TemplatePlacingError::Selector {
                    entry: "workspace".into(),
                    source: tastemaker_values::domain::selectors::errors::SelectorsError::Pattern {
                        pattern: "(".into(),
                        reason: "regex parse error:\n    (\n    ^\nerror: unclosed group".into(),
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    MatchingGlobsetAdapter::default().compile(spelling)
}

/// The values `spelling` binds in `path`, empty when they are `greeting`.
fn bound(spelling: &Spelling, path: &str) -> Result<Value, Failure> {
    let test = compile(spelling).map_err(|_refused| Refused::Pattern)?;
    let bound = test
        .bind(PathRest(path))
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.0.to_string())
        .collect::<Vec<_>>();
    Ok(Answer::Bound(if bound == ["greeting"] {
        Vec::new()
    } else {
        bound
    }))
}

fn expected(question: Question) -> &'static str {
    match question {
        Question::Matches("crates/core/src") => "crate = core",
        Question::Matches(_) => "member = c, parent = a/b",
        Question::Covers("crates/core/src/lib.txt") => "Covered",
        Question::Covers(_) => "Uncovered",
        Question::Leading => "folder",
        Question::Group(..) => "crates/ryo",
        Question::Variable => "",
    }
}

/// What a pattern placed at `base` answers, empty when it is what the question expects.
fn selected(base: &str, pattern: &str, question: Question) -> Result<Value, Failure> {
    let base = RepoPath(base.into());
    let values = Interpolations::default();
    let matcher = place(&base, &PathPattern(pattern.into()), &values, &compile)
        .map_err(|_refused| Refused::Pattern)?;
    let answer = match question {
        Question::Matches(path) => match matches(&matcher, &RepoPath(path.into())) {
            Match::Matched(captures) => {
                tastemaker_values::domain::captures::service::captures_text(&captures).0
            }
            Match::Missed => return Err(Refused::Pattern),
        },
        Question::Covers(path) => format!("{:?}", covers(&matcher, &RepoPath(path.into()))),
        Question::Leading => leading(&matcher)
            .into_iter()
            .map(|name| name.0.into_string())
            .collect::<Vec<_>>()
            .join(","),
        Question::Group(name, value) => {
            let mut captures = Captures::default();
            drop(
                captures
                    .0
                    .insert(CaptureName(name.into()), CaptureValue(value.into())),
            );
            group(&matcher, &captures)
                .ok_or(Refused::Pattern)?
                .0
                .into_string()
        }
        Question::Variable => {
            let list = Selectors(vec![Selector::Variable(VariableName("crates".into()))]);
            return match resolve(&list, &context(&base, &values)).err() {
                Some(
                    tastemaker_values::domain::selectors::errors::SelectorsError::Unresolved {
                        ..
                    },
                ) => Err(Refused::Pattern),
                Some(tastemaker_values::domain::selectors::errors::SelectorsError::Pattern {
                    reason,
                    ..
                }) => Ok(Answer::Said(reason)),
                None => Ok(Answer::Said("resolved".to_owned())),
            };
        }
    };
    Ok(Answer::Said(if answer == expected(question) {
        String::new()
    } else {
        answer
    }))
}

fn context<'a>(base: &'a RepoPath, values: &'a Interpolations) -> SelectionContext<'a> {
    SelectionContext {
        base,
        values,
        instances: &[],
        placed: &[],
        compile: &compile,
    }
}

/// Whether a regex selector written for `crate` covers `path`.
fn regex(expression: &str, path: &str) -> Result<Value, Failure> {
    let base = RepoPath("crate".into());
    let values = Interpolations::default();
    let resolved = resolve(
        &Selectors(vec![Selector::Regex(RegexText(expression.into()))]),
        &context(&base, &values),
    )
    .map_err(|_refused| Refused::Pattern)?;
    Ok(Answer::Covered(
        covers_any(&resolved, &RepoPath(path.into())) == Coverage::Covered,
    ))
}

/// The files of `docs/a.md` and `src/lib.txt` a `rules-exclude` of `selector` covers, empty when it is `docs/a.md`.
fn excluded(selector: Selector) -> Result<Value, Failure> {
    let mut workspace = bare(&["docs/a.md", "src/lib.txt"]);
    workspace.section.rules_exclude = Selectors(vec![selector]);
    let said = exclusion(&workspace, &compile, &[])
        .map_err(Refused::Placing)?
        .iter()
        .map(|path| path.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(Answer::Said(if said == "docs/a.md" {
        String::new()
    } else {
        said
    }))
}

fn subject(row: Input) -> Result<Value, Failure> {
    match row {
        Row::Bind(spelling, path) => bound(&spelling, path),
        Row::Selector(base, pattern, question) => selected(base, pattern, question),
        Row::Regex(expression, path) => regex(expression, path),
        Row::Exclude(selector) => excluded(selector),
    }
}

#[test]
fn succeeds() {
    check(&MatchingGlobsetAdapter::default());
    run(CASES, Word::Succeeds, subject);
    assert_eq!(Coverage::from(true), Coverage::Covered);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
