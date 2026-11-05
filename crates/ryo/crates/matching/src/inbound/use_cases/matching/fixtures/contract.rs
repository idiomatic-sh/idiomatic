//! Contract subject uses only the public semantic trait.
use super::super::errors::MatchingError;
use super::super::interface::MatchingUseCase;
use crate::cases::{Case, CaseName, Expect, Word};
use std::ops::ControlFlow;

#[derive(Debug)]
pub enum MatchingInput {
    Regex(&'static str, &'static str),
    Globs(&'static [&'static str], &'static str),
    Parse(&'static str),
    Ranges(&'static str, &'static str),
    Captures(&'static str, &'static str, &'static str),
    StopRanges(usize),
    StopCaptures(usize),
    TextGlob(&'static str, &'static str),
    TextGlobSet(&'static [&'static str], &'static str),
    DeepTextGlob,
    OwnedTextGlobs,
}

#[derive(Debug, PartialEq)]
pub enum MatchingValue {
    Match(bool),
    Hits(Vec<usize>),
    Parsed,
    Ranges(Vec<std::ops::Range<usize>>),
    Captures(Vec<(std::ops::Range<usize>, std::ops::Range<usize>)>),
}

type Input = MatchingInput;
type Value = MatchingValue;
type Failure = MatchingError;

pub fn answer(matching: &impl MatchingUseCase, input: Input) -> Result<Value, MatchingError> {
    match input {
        Input::Regex(pattern, text) => {
            let regex = matching.compile_regex(pattern)?;
            let first = matching.matches_regex(&regex, text);
            assert_eq!(matching.matches_regex(&regex.clone(), text), first);
            Ok(Value::Match(first))
        }
        Input::Globs(patterns, path) => {
            let globs = matching.compile_globs(patterns)?;
            let mut hits = vec![usize::MAX];
            matching.match_globs(&globs, std::path::Path::new(path), &mut hits);
            let first = hits.clone();
            matching.match_globs(&globs.clone(), std::path::Path::new(path), &mut hits);
            assert_eq!(hits, first);
            Ok(Value::Hits(hits))
        }
        Input::Ranges(pattern, text) => {
            let regex = matching.compile_regex(pattern)?;
            let mut ranges = Vec::new();
            let flow = matching.visit_regex_matches(&regex, text, &mut |range| {
                ranges.push(range);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Continue(()));
            Ok(Value::Ranges(ranges))
        }
        Input::Captures(pattern, text, name) => {
            let regex = matching.compile_regex(pattern)?;
            let mut ranges = Vec::new();
            let result = matching.visit_regex_captures(&regex, text, name, &mut |whole, group| {
                ranges.push((whole, group));
                ControlFlow::Continue(())
            });
            if result.is_err() {
                assert!(ranges.is_empty(), "invalid capture called the visitor");
            }
            assert_eq!(result?, ControlFlow::Continue(()));
            Ok(Value::Captures(ranges))
        }
        Input::StopRanges(stop_at) => {
            let regex = matching.compile_regex("a")?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow = matching.visit_regex_matches(&regex, "a a a", &mut |range| {
                visits += 1;
                if visits == stop_at {
                    failure = Some("delivery failed");
                    return ControlFlow::Break(());
                }
                ranges.push(range);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(Value::Ranges(ranges))
        }
        Input::StopCaptures(stop_at) => {
            let regex = matching.compile_regex("(?P<letter>a)")?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow =
                matching.visit_regex_captures(&regex, "a a a", "letter", &mut |whole, group| {
                    visits += 1;
                    if visits == stop_at {
                        failure = Some("delivery failed");
                        return ControlFlow::Break(());
                    }
                    ranges.push((whole, group));
                    ControlFlow::Continue(())
                })?;
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(Value::Captures(ranges))
        }
        Input::TextGlob(pattern, text) => {
            let direct = matching.matches_text_glob(pattern, text);
            let compiled = matching.compile_text_globs(&[pattern]);
            assert_eq!(matching.matches_text_globs(&compiled, text), direct);
            Ok(Value::Match(direct))
        }
        Input::TextGlobSet(patterns, text) => {
            let compiled = matching.compile_text_globs(patterns);
            Ok(Value::Match(matching.matches_text_globs(&compiled, text)))
        }
        Input::DeepTextGlob => {
            let pattern = format!("{}a{}", "{".repeat(300), "}".repeat(300));
            let compiled = matching.compile_text_globs(&[&pattern]);
            Ok(Value::Match(matching.matches_text_globs(&compiled, "a")))
        }
        Input::OwnedTextGlobs => {
            let compiled = {
                let pattern = String::from("a?c");
                matching.compile_text_globs(&[&pattern])
            };
            Ok(Value::Match(matching.matches_text_globs(&compiled, "aéc")))
        }
        Input::Parse(pattern) => matching.parse_glob(pattern).map(|()| Value::Parsed),
    }
}

pub fn cases() -> Vec<Case<Input, Value, Failure>> {
    vec![
        Case {
            name: CaseName("text_star_crosses_separator"),
            word: Word::Succeeds,
            input: || Input::TextGlob("src/*.rs", "src/a/b.rs"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("path_star_preserves_separator"),
            word: Word::Succeeds,
            input: || Input::Globs(&["src/*.rs"], "src/a/b.rs"),
            expect: Expect::Value(Value::Hits(vec![])),
        },
        Case {
            name: CaseName("text_star_crosses_newlines"),
            word: Word::Succeeds,
            input: || Input::TextGlob("a*b", "a\nb"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_question_consumes_unicode_character"),
            word: Word::Succeeds,
            input: || Input::TextGlob("a?c", "aéc"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_question_does_not_consume_two_characters"),
            word: Word::Succeeds,
            input: || Input::TextGlob("a?c", "aééc"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_directory_star_zero_segments"),
            word: Word::Succeeds,
            input: || Input::TextGlob("src/**/lib.rs", "src/lib.rs"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_directory_star_requires_complete_segment"),
            word: Word::Succeeds,
            input: || Input::TextGlob("src/**/lib.rs", "src/xlib.rs"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_negated_class_refuses_member"),
            word: Word::Succeeds,
            input: || Input::TextGlob("[!a-c]x", "bx"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_escape_preserves_star_literal"),
            word: Word::Succeeds,
            input: || Input::TextGlob("a\\*", "a*"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_escape_refuses_wildcard_interpretation"),
            word: Word::Succeeds,
            input: || Input::TextGlob("a\\*", "ab"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_braces_nest"),
            word: Word::Succeeds,
            input: || Input::TextGlob("{a,{b,c}}x", "cx"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_empty_alternative"),
            word: Word::Succeeds,
            input: || Input::TextGlob("{a,}", ""),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("malformed_text_brace_is_literal"),
            word: Word::Succeeds,
            input: || Input::TextGlob("{a", "{a"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("malformed_text_brace_refuses_other_text"),
            word: Word::Succeeds,
            input: || Input::TextGlob("{a", "a"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("path_glob_still_refuses_malformed_brace"),
            word: Word::Refuses,
            input: || Input::Parse("{a"),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::BadGlob { glob, reason } if glob == "{a" && !reason.is_empty()),
                )
            }),
        },
        Case {
            name: CaseName("text_set_uses_interpreter_for_whole_set"),
            word: Word::Succeeds,
            input: || Input::TextGlobSet(&["literal", "a?c"], "aéc"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_set_negative"),
            word: Word::Succeeds,
            input: || Input::TextGlobSet(&["literal", "a?c"], "aééc"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_set_empty"),
            word: Word::Succeeds,
            input: || Input::TextGlobSet(&[], "anything"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("text_compiler_limit_keeps_semantics"),
            word: Word::Succeeds,
            input: || Input::DeepTextGlob,
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("text_interpreter_owns_authored_pattern"),
            word: Word::Succeeds,
            input: || Input::OwnedTextGlobs,
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("source_order_unicode_ranges"),
            word: Word::Succeeds,
            input: || Input::Ranges("é|x", "é x é"),
            expect: Expect::Value(Value::Ranges(vec![0..2, 3..4, 5..7])),
        },
        Case {
            name: CaseName("zero_width_ranges_terminate"),
            word: Word::Succeeds,
            input: || Input::Ranges("", "ab"),
            expect: Expect::Value(Value::Ranges(vec![0..0, 1..1, 2..2])),
        },
        Case {
            name: CaseName("optional_named_group_skips_absent"),
            word: Word::Succeeds,
            input: || Input::Captures("(?P<letter>a)?b", "b ab", "letter"),
            expect: Expect::Value(Value::Captures(vec![(2..4, 2..3)])),
        },
        Case {
            name: CaseName("unknown_named_group_refuses_before_matching"),
            word: Word::Refuses,
            input: || Input::Captures("a", "a", "absent"),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::InvalidRegex { reason } if reason == "missing named capture `absent`"),
                )
            }),
        },
        Case {
            name: CaseName("unknown_named_group_refuses_on_empty_text"),
            word: Word::Refuses,
            input: || Input::Captures("a", "", "absent"),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::InvalidRegex { reason } if reason == "missing named capture `absent`"),
                )
            }),
        },
        Case {
            name: CaseName("known_optional_group_all_unmatched"),
            word: Word::Succeeds,
            input: || Input::Captures("(?P<letter>a)?b", "b b", "letter"),
            expect: Expect::Value(Value::Captures(vec![])),
        },
        Case {
            name: CaseName("range_failure_preserves_prefix"),
            word: Word::Succeeds,
            input: || Input::StopRanges(2),
            expect: Expect::Value(Value::Ranges(vec![0..1])),
        },
        Case {
            name: CaseName("range_first_failure_emits_nothing"),
            word: Word::Succeeds,
            input: || Input::StopRanges(1),
            expect: Expect::Value(Value::Ranges(vec![])),
        },
        Case {
            name: CaseName("capture_failure_preserves_prefix"),
            word: Word::Succeeds,
            input: || Input::StopCaptures(2),
            expect: Expect::Value(Value::Captures(vec![(0..1, 0..1)])),
        },
        Case {
            name: CaseName("capture_first_failure_emits_nothing"),
            word: Word::Succeeds,
            input: || Input::StopCaptures(1),
            expect: Expect::Value(Value::Captures(vec![])),
        },
        Case {
            name: CaseName("regex_directive"),
            word: Word::Succeeds,
            input: || Input::Regex("(?i)^ *noqa: *E722(?:[, ]|$)", " NOQA: E722, F401"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("regex_code_prefix_is_not_code"),
            word: Word::Succeeds,
            input: || Input::Regex("(?i)^ *noqa: *E722(?:[, ]|$)", "noqa: E7220"),
            expect: Expect::Value(Value::Match(false)),
        },
        Case {
            name: CaseName("unicode_regex"),
            word: Word::Succeeds,
            input: || Input::Regex("^.$", "é"),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("empty_regex"),
            word: Word::Succeeds,
            input: || Input::Regex("^$", ""),
            expect: Expect::Value(Value::Match(true)),
        },
        Case {
            name: CaseName("regex_refuses"),
            word: Word::Refuses,
            input: || Input::Regex("[", "text"),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::InvalidRegex { reason } if !reason.is_empty()),
                )
            }),
        },
        Case {
            name: CaseName("glob_root_and_authored_order"),
            word: Word::Succeeds,
            input: || Input::Globs(&["*.rs", "**/*.rs", "**/Cargo.lock", "*.rs"], "lib.rs"),
            expect: Expect::Value(Value::Hits(vec![0, 1, 3])),
        },
        Case {
            name: CaseName("single_star_does_not_cross_separator"),
            word: Word::Succeeds,
            input: || Input::Globs(&["*.rs", "**/*.rs"], "src/lib.rs"),
            expect: Expect::Value(Value::Hits(vec![1])),
        },
        Case {
            name: CaseName("glob_zero_directory_segments"),
            word: Word::Succeeds,
            input: || Input::Globs(&["**/Cargo.lock"], "Cargo.lock"),
            expect: Expect::Value(Value::Hits(vec![0])),
        },
        Case {
            name: CaseName("glob_negative_clears_prior_hits"),
            word: Word::Succeeds,
            input: || Input::Globs(&["**/*.rs"], "src/lib.txt"),
            expect: Expect::Value(Value::Hits(vec![])),
        },
        Case {
            name: CaseName("empty_set"),
            word: Word::Succeeds,
            input: || Input::Globs(&[], "lib.rs"),
            expect: Expect::Value(Value::Hits(vec![])),
        },
        Case {
            name: CaseName("first_bad_glob"),
            word: Word::Refuses,
            input: || Input::Globs(&["*.rs", "[", "{"], "lib.rs"),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
                )
            }),
        },
        Case {
            name: CaseName("parse_valid_glob"),
            word: Word::Succeeds,
            input: || Input::Parse("**/*.rs"),
            expect: Expect::Value(Value::Parsed),
        },
        Case {
            name: CaseName("parse_bad_glob"),
            word: Word::Refuses,
            input: || Input::Parse("["),
            expect: Expect::Refusal(|error| {
                crate::cases::selects(
                    matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
                )
            }),
        },
    ]
}

/// Run all nonempty positive and refusal rows against a production implementation.
pub fn check(subject: &impl MatchingUseCase) {
    let cases = cases();
    crate::cases::run(&cases, crate::cases::Word::Succeeds, |input| {
        answer(subject, input)
    });
    crate::cases::run(&cases, crate::cases::Word::Refuses, |input| {
        answer(subject, input)
    });
}
