//! What every implementation of the `matching` use case's trait must pass.

use super::super::errors::MatchingError;
use super::super::interface::MatchingUseCase;
use super::subject::{MatchingInput, MatchingValue, answer};
use crate::cases::{Case, CaseName, Expect, Word};
use std::borrow::Cow;

/// What a row of the contract is given.
type Input = MatchingInput;
/// What a row of the contract expects when it succeeds.
type Value = MatchingValue;
/// What a row of the contract expects when it is refused.
type Failure = MatchingError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_star_crosses_separator"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("src/*.rs", "src/a/b.rs"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("path_star_preserves_separator"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&["src/*.rs"], "src/a/b.rs"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("text_star_crosses_newlines"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("a*b", "a\nb"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_question_consumes_unicode_character"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("a?c", "aéc"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_question_does_not_consume_two_characters"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("a?c", "aééc"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_directory_star_zero_segments"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("src/**/lib.rs", "src/lib.rs"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_directory_star_requires_complete_segment"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("src/**/lib.rs", "src/xlib.rs"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_negated_class_refuses_member"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("[!a-c]x", "bx"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_escape_preserves_star_literal"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("a\\*", "a*"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_escape_refuses_wildcard_interpretation"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("a\\*", "ab"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_braces_nest"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("{a,{b,c}}x", "cx"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_empty_alternative"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("{a,}", ""),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("malformed_text_brace_is_literal"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("{a", "{a"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("malformed_text_brace_refuses_other_text"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlob("{a", "a"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("path_glob_still_refuses_malformed_brace"),
        word: Word::Refuses,
        input: || MatchingInput::Parse("{a"),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::BadGlob { glob, reason } if glob == "{a" && !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("text_set_uses_interpreter_for_whole_set"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlobSet(&["literal", "a?c"], "aéc"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_set_negative"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlobSet(&["literal", "a?c"], "aééc"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_set_empty"),
        word: Word::Succeeds,
        input: || MatchingInput::TextGlobSet(&[], "anything"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("text_compiler_limit_keeps_semantics"),
        word: Word::Succeeds,
        input: || MatchingInput::DeepTextGlob,
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("text_interpreter_owns_authored_pattern"),
        word: Word::Succeeds,
        input: || MatchingInput::OwnedTextGlobs,
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("source_order_unicode_ranges"),
        word: Word::Succeeds,
        input: || MatchingInput::Ranges("é|x", "é x é"),
        expect: Expect::Value(MatchingValue::Ranges(Cow::Borrowed(&[0..2, 3..4, 5..7]))),
    },
    Case {
        name: CaseName("zero_width_ranges_terminate"),
        word: Word::Succeeds,
        input: || MatchingInput::Ranges("", "ab"),
        expect: Expect::Value(MatchingValue::Ranges(Cow::Borrowed(&[0..0, 1..1, 2..2]))),
    },
    Case {
        name: CaseName("optional_named_group_skips_absent"),
        word: Word::Succeeds,
        input: || MatchingInput::Captures("(?P<letter>a)?b", "b ab", "letter"),
        expect: Expect::Value(MatchingValue::Captures(Cow::Borrowed(&[(2..4, 2..3)]))),
    },
    Case {
        name: CaseName("unknown_named_group_refuses_before_matching"),
        word: Word::Refuses,
        input: || MatchingInput::Captures("a", "a", "absent"),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::InvalidRegex { reason } if reason == "missing named capture `absent`"),
            )
        }),
    },
    Case {
        name: CaseName("unknown_named_group_refuses_on_empty_text"),
        word: Word::Refuses,
        input: || MatchingInput::Captures("a", "", "absent"),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::InvalidRegex { reason } if reason == "missing named capture `absent`"),
            )
        }),
    },
    Case {
        name: CaseName("known_optional_group_all_unmatched"),
        word: Word::Succeeds,
        input: || MatchingInput::Captures("(?P<letter>a)?b", "b b", "letter"),
        expect: Expect::Value(MatchingValue::Captures(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("range_failure_preserves_prefix"),
        word: Word::Succeeds,
        input: || MatchingInput::StopRanges(2),
        expect: Expect::Value(MatchingValue::Ranges(Cow::Borrowed(&[0..1]))),
    },
    Case {
        name: CaseName("range_first_failure_emits_nothing"),
        word: Word::Succeeds,
        input: || MatchingInput::StopRanges(1),
        expect: Expect::Value(MatchingValue::Ranges(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("capture_failure_preserves_prefix"),
        word: Word::Succeeds,
        input: || MatchingInput::StopCaptures(2),
        expect: Expect::Value(MatchingValue::Captures(Cow::Borrowed(&[(0..1, 0..1)]))),
    },
    Case {
        name: CaseName("capture_first_failure_emits_nothing"),
        word: Word::Succeeds,
        input: || MatchingInput::StopCaptures(1),
        expect: Expect::Value(MatchingValue::Captures(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("regex_directive"),
        word: Word::Succeeds,
        input: || MatchingInput::Regex("(?i)^ *noqa: *E722(?:[, ]|$)", " NOQA: E722, F401"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("regex_code_prefix_is_not_code"),
        word: Word::Succeeds,
        input: || MatchingInput::Regex("(?i)^ *noqa: *E722(?:[, ]|$)", "noqa: E7220"),
        expect: Expect::Value(MatchingValue::Match(false)),
    },
    Case {
        name: CaseName("unicode_regex"),
        word: Word::Succeeds,
        input: || MatchingInput::Regex("^.$", "é"),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("empty_regex"),
        word: Word::Succeeds,
        input: || MatchingInput::Regex("^$", ""),
        expect: Expect::Value(MatchingValue::Match(true)),
    },
    Case {
        name: CaseName("regex_refuses"),
        word: Word::Refuses,
        input: || MatchingInput::Regex("[", "text"),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::InvalidRegex { reason } if !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("glob_root_and_authored_order"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&["*.rs", "**/*.rs", "**/Cargo.lock", "*.rs"], "lib.rs"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[0, 1, 3]))),
    },
    Case {
        name: CaseName("single_star_does_not_cross_separator"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&["*.rs", "**/*.rs"], "src/lib.rs"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("glob_zero_directory_segments"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&["**/Cargo.lock"], "Cargo.lock"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[0]))),
    },
    Case {
        name: CaseName("glob_negative_clears_prior_hits"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&["**/*.rs"], "src/lib.txt"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("empty_set"),
        word: Word::Succeeds,
        input: || MatchingInput::Globs(&[], "lib.rs"),
        expect: Expect::Value(MatchingValue::Hits(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("first_bad_glob"),
        word: Word::Refuses,
        input: || MatchingInput::Globs(&["*.rs", "[", "{"], "lib.rs"),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("parse_valid_glob"),
        word: Word::Succeeds,
        input: || MatchingInput::Parse("**/*.rs"),
        expect: Expect::Value(MatchingValue::Parsed),
    },
    Case {
        name: CaseName("parse_bad_glob"),
        word: Word::Refuses,
        input: || MatchingInput::Parse("["),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
            )
        }),
    },
];

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
pub fn check(subject: &impl MatchingUseCase) {
    crate::cases::run(CASES, crate::cases::Word::Succeeds, |input| {
        answer(subject, input)
    });
    crate::cases::run(CASES, crate::cases::Word::Refuses, |input| {
        answer(subject, input)
    });
}
