//! What the `selectors` service answers of placed patterns: what they match, cover, capture and group.

use std::sync::Arc;

use super::errors::SelectorsError;
use super::service::{covers, group, leading, matches, place, resolve};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::{CaptureName, CaptureValue, Captures, Interpolations};
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::{
    Coverage, Match, PathPattern, PathTest, PatternFailure, SelectionContext, Selector, Selectors,
    Spelling, VariableName,
};
use crate::outbound::matching::factory::globset;
use crate::outbound::matching::interface::MatchingPort as _;

/// A pattern placed at a base, and the question asked of it.
type Input = (&'static str, &'static str, Question);
/// The answer, as text.
type Value = String;
type Failure = ();

/// A question asked of a placed pattern.
#[derive(Clone, Copy, Debug)]
enum Question {
    Matches(&'static str),
    Covers(&'static str),
    Leading,
    Group(&'static str, &'static str),
    /// A list that still holds a variable, resolved.
    Variable,
}

fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    globset().compile(spelling)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_capture_binds_one_segment"),
        word: Word::Succeeds,
        input: || {
            (
                "crates",
                "{crate}/src",
                Question::Matches("crates/core/src"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_span_capture_binds_whole_segments"),
        word: Word::Succeeds,
        input: || ("", "{*parent}/{member}", Question::Matches("a/b/c")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_pattern_covers_what_lies_beneath_its_match"),
        word: Word::Succeeds,
        input: || ("", "crates/*", Question::Covers("crates/core/src/lib.txt")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_pattern_does_not_cover_a_sibling_with_its_prefix"),
        word: Word::Succeeds,
        input: || ("", "crates/core", Question::Covers("crates/core2/lib.txt")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_names_before_the_last_capture_lead"),
        word: Word::Succeeds,
        input: || ("", "crates/{folder}/{member}", Question::Leading),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_group_is_the_directory_of_the_last_capture"),
        word: Word::Succeeds,
        input: || {
            (
                "",
                "crates/{folder}/{member}",
                Question::Group("folder", "ryo"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_bound_twice_with_two_values_misses"),
        word: Word::Refuses,
        input: || ("", "{x}/{x}", Question::Matches("a/b")),
        expect: Expect::error(()),
    },
    Case {
        name: CaseName("a_variable_left_in_a_list_is_refused_at_placement"),
        word: Word::Refuses,
        input: || ("", "src", Question::Variable),
        expect: Expect::error(()),
    },
];

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

fn subject((base, pattern, question): Input) -> Result<Value, Failure> {
    let matcher = place(
        &RepoPath(base.into()),
        &PathPattern(pattern.into()),
        &Interpolations::default(),
        &compile,
    )
    .map_err(drop)?;
    let answer = match question {
        Question::Matches(path) => match matches(&matcher, &RepoPath(path.into())) {
            Match::Matched(captures) => {
                crate::domain::captures::service::captures_text(&captures).0
            }
            Match::Missed => return Err(()),
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
            group(&matcher, &captures).ok_or(())?.0.into_string()
        }
        Question::Variable => {
            let base = RepoPath(base.into());
            let values = Interpolations::default();
            let context = SelectionContext {
                base: &base,
                values: &values,
                instances: &[],
                placed: &[],
                compile: &compile,
            };
            let list = Selectors(vec![Selector::Variable(VariableName("crates".into()))]);
            return match resolve(&list, &context).err() {
                Some(SelectorsError::Unresolved { .. }) => Err(()),
                Some(SelectorsError::Pattern { reason, .. }) => Ok(reason),
                None => Ok("resolved".to_owned()),
            };
        }
    };
    Ok(if answer == expected(question) {
        String::new()
    } else {
        answer
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(Coverage::from(true), Coverage::Covered);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
