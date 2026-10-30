//! The `globbing` service's cases, on values alone.

use super::service::{compile, matches, matches_compiled};
use super::types::{GlobMatch, GlobText, Subject};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A glob and the text it is matched against.
type Input = (&'static str, &'static str);
/// Whether the glob matches all of the text.
type Value = GlobMatch;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_that_looks_like_a_parent_path_keeps_its_last_dot"),
        word: Word::Succeeds,
        input: || ("*.", ".."),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_backslash_subject_is_not_a_forward_slash_path"),
        word: Word::Succeeds,
        input: || ("a/b", "a\\b"),
        expect: Expect::Value(GlobMatch::Differs),
    },
    Case {
        name: CaseName("literal_alternatives_keep_empty_members"),
        word: Word::Succeeds,
        input: || ("pre{,é}post", "prepost"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("empty_alternatives_match_empty_text"),
        word: Word::Succeeds,
        input: || ("{a,}", ""),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("nested_empty_alternatives_match_empty_text"),
        word: Word::Succeeds,
        input: || ("{a,{,b}}", ""),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("ordinary_stars_keep_unicode_subjects"),
        word: Word::Succeeds,
        input: || ("a*b", "aé/b"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("ordinary_stars_cross_newlines"),
        word: Word::Succeeds,
        input: || ("a*b", "a\nb"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("malformed_braces_retain_literal_matching"),
        word: Word::Succeeds,
        input: || ("{a", "{a"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("question_marks_take_unicode_characters"),
        word: Word::Succeeds,
        input: || ("a?c", "aéc"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_star_crosses_directories"),
        word: Word::Succeeds,
        input: || ("src/*.rs", "src/a/b.rs"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_double_star_slash_may_be_no_directory"),
        word: Word::Succeeds,
        input: || ("src/**/lib.rs", "src/lib.rs"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_double_star_slash_keeps_the_names_whole"),
        word: Word::Succeeds,
        input: || ("src/**/lib.rs", "src/xlib.rs"),
        expect: Expect::Value(GlobMatch::Differs),
    },
    Case {
        name: CaseName("a_question_mark_is_one_character"),
        word: Word::Succeeds,
        input: || ("a?c", "abc"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_class_takes_one_of_its_range"),
        word: Word::Succeeds,
        input: || ("[a-c]x", "bx"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_negated_class_refuses_its_range"),
        word: Word::Succeeds,
        input: || ("[!a-c]x", "bx"),
        expect: Expect::Value(GlobMatch::Differs),
    },
    Case {
        name: CaseName("a_bracket_right_after_the_opening_is_a_member"),
        word: Word::Succeeds,
        input: || ("[]]", "]"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_backslash_makes_a_star_itself"),
        word: Word::Succeeds,
        input: || ("a\\*", "a*"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("an_escaped_star_matches_no_run"),
        word: Word::Succeeds,
        input: || ("a\\*", "ab"),
        expect: Expect::Value(GlobMatch::Differs),
    },
    Case {
        name: CaseName("one_alternative_of_a_group_matches"),
        word: Word::Succeeds,
        input: || ("src/{a,b}.rs", "src/b.rs"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("no_alternative_of_a_group_differs"),
        word: Word::Succeeds,
        input: || ("src/{a,b}.rs", "src/c.rs"),
        expect: Expect::Value(GlobMatch::Differs),
    },
    Case {
        name: CaseName("groups_may_follow_one_another"),
        word: Word::Succeeds,
        input: || ("{a,b}{c,d}", "bd"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("groups_may_nest"),
        word: Word::Succeeds,
        input: || ("{a,{b,c}}x", "cx"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_brace_inside_a_class_is_a_member"),
        word: Word::Succeeds,
        input: || ("[{]x", "{x"),
        expect: Expect::Value(GlobMatch::Matches),
    },
    Case {
        name: CaseName("a_comma_inside_a_class_splits_no_alternative"),
        word: Word::Succeeds,
        input: || ("{[,]a,b}", ",a"),
        expect: Expect::Value(GlobMatch::Matches),
    },
];

fn subject((glob, text): Input) -> Value {
    let interpreted = matches(GlobText(glob), Subject(text));
    if let Some(compiled) = matches_compiled(&compile(&[GlobText(glob)]), Subject(text)) {
        assert_eq!(compiled, interpreted);
    }
    interpreted
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
    // A valid glob can exceed the library regex parser's nesting limit. Compilation
    // must decline it rather than panic or change the interpreter's answer.
    let deep = format!("{}a{}", "{".repeat(300), "}".repeat(300));
    assert_eq!(matches(GlobText(&deep), Subject("a")), GlobMatch::Matches);
    assert_eq!(
        matches_compiled(&compile(&[GlobText(&deep)]), Subject("a")),
        None
    );
    assert_eq!(
        matches_compiled(&compile(&[]), Subject("anything")),
        Some(GlobMatch::Differs)
    );
}
