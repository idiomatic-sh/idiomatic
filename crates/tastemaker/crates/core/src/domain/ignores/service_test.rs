//! What the `ignores` service reads and applies: each row a file and one finding of `p/r` in it, and the outcome written
//! by hand: what the file's first ignore reaches, and how many findings are kept, suppressed, and problems.

use std::collections::BTreeSet;

use super::service::{apply, ignores, read};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::comments::service::{comment, lines};
use crate::domain::comments::types::{
    ByteSpan, Comment, CommentKind, CommentText, Documentation, LineIndex,
};
use crate::domain::findings::service::{finding, spanning};
use crate::domain::findings::types::RuleId;
use crate::domain::findings::types::{ByteOffset, SourceText};
use crate::domain::findings::types::{Help, Place};
use crate::domain::ignores::types::Covered;
use crate::domain::ignores::types::Sifted;
use crate::domain::paths::types::RepoPath;

/// A file's text, and the line of the one finding of `p/r` in it (0 for the whole file).
type Input = (&'static str, u32);

/// What the service makes of a row's file and finding.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// What the file's first usable ignore reaches; none when it writes no usable ignore.
    reach: Option<Covered>,
    /// The findings kept.
    kept: usize,
    /// The findings suppressed.
    suppressed: usize,
    /// The malformed and unused ignores.
    problems: usize,
}

/// The file's comments read no ignore at all.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    NotAnIgnore,
}

/// What a row expects: the outcome, or that the file reads no ignore.
type Value = Outcome;
type Failure = Refusal;

const fn lines_of(first: u32, last: u32) -> Covered {
    Covered::Lines {
        first: LineIndex(first),
        last: LineIndex(last),
    }
}

const SUPPRESSED: Outcome = Outcome {
    reach: None,
    kept: 0,
    suppressed: 1,
    problems: 0,
};

const UNUSED: Outcome = Outcome {
    reach: None,
    kept: 1,
    suppressed: 0,
    problems: 1,
};

const CASES: &[Case<Input, Value, Failure>] = &[
    // Covered::File
    Case {
        name: CaseName("an_ignore_above_all_code_covers_a_finding_about_the_whole_file"),
        word: Word::Succeeds,
        input: || ("# ignore: p/r - the file is generated\nkey = 1\n", 0),
        expect: Expect::Value(Outcome {
            reach: Some(Covered::File),
            ..SUPPRESSED
        }),
    },
    Case {
        name: CaseName("an_ignore_above_all_code_covers_a_finding_on_any_line"),
        word: Word::Succeeds,
        input: || {
            (
                "\n# ignore: p/r - the file is generated\n\nkey = 1\nother = 2\n",
                5,
            )
        },
        expect: Expect::Value(Outcome {
            reach: Some(Covered::File),
            ..SUPPRESSED
        }),
    },
    // Covered::Lines, written at the end of a line
    Case {
        name: CaseName("an_ignore_at_the_end_of_a_line_covers_that_line"),
        word: Word::Succeeds,
        input: || ("key = 1\nother = 2 # ignore: p/r - a legacy key\n", 2),
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(2, 2)),
            ..SUPPRESSED
        }),
    },
    Case {
        name: CaseName("an_ignore_at_the_end_of_a_line_leaves_another_line_and_is_unused"),
        word: Word::Succeeds,
        input: || ("key = 1\nother = 2 # ignore: p/r - a legacy key\n", 1),
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(2, 2)),
            ..UNUSED
        }),
    },
    // Covered::Lines, written on its own line after code: the next item
    Case {
        name: CaseName("an_ignore_on_its_own_line_covers_the_next_item_and_its_deeper_lines"),
        word: Word::Succeeds,
        input: || {
            (
                "a = 1\n# ignore: p/r - a legacy table\n[t]\n  x = 1\nb = 2\n",
                4,
            )
        },
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(3, 4)),
            ..SUPPRESSED
        }),
    },
    Case {
        name: CaseName("an_ignore_on_its_own_line_leaves_a_finding_shown_on_the_comment_below_it"),
        word: Word::Succeeds,
        input: || {
            (
                "a = 1\n# ignore: p/r - a legacy table\n# a note\nb = 2\n",
                3,
            )
        },
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(4, 4)),
            ..UNUSED
        }),
    },
    Case {
        name: CaseName("an_ignore_on_its_own_line_leaves_the_item_after_the_next"),
        word: Word::Succeeds,
        input: || {
            (
                "a = 1\n# ignore: p/r - a legacy table\n[t]\n  x = 1\nb = 2\n",
                5,
            )
        },
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(3, 4)),
            ..UNUSED
        }),
    },
    Case {
        name: CaseName("an_ignore_on_its_own_line_covers_the_line_that_closes_the_item"),
        word: Word::Succeeds,
        input: || {
            (
                "a = 1\n# ignore: p/r - a legacy list\nb = [\n  1,\n]\nc = 2\n",
                5,
            )
        },
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(3, 5)),
            ..SUPPRESSED
        }),
    },
    Case {
        name: CaseName("an_ignore_on_its_own_line_leaves_a_finding_about_the_whole_file"),
        word: Word::Succeeds,
        input: || ("a = 1\n# ignore: p/r - a legacy key\nb = 2\n", 0),
        expect: Expect::Value(Outcome {
            reach: Some(lines_of(3, 3)),
            ..UNUSED
        }),
    },
    // Covered::Nothing
    Case {
        name: CaseName("an_ignore_with_no_code_after_it_covers_nothing"),
        word: Word::Succeeds,
        input: || ("a = 1\n# ignore: p/r - nothing follows\n\n", 1),
        expect: Expect::Value(Outcome {
            reach: Some(Covered::Nothing),
            ..UNUSED
        }),
    },
    // Written::NoRule, Written::NoReason, and a Written::Ignore of a rule not loaded
    Case {
        name: CaseName("an_ignore_naming_no_rule_is_a_problem_and_suppresses_nothing"),
        word: Word::Succeeds,
        input: || ("# ignore: - a reason\nkey = 1\n", 2),
        expect: Expect::Value(UNUSED),
    },
    Case {
        name: CaseName("an_ignore_with_no_reason_is_a_problem_and_suppresses_nothing"),
        word: Word::Succeeds,
        input: || ("# ignore: p/r\nkey = 1\n", 2),
        expect: Expect::Value(UNUSED),
    },
    Case {
        name: CaseName("an_ignore_of_a_rule_not_loaded_is_a_problem_and_suppresses_nothing"),
        word: Word::Succeeds,
        input: || ("# ignore: q/z - a reason\nkey = 1\n", 2),
        expect: Expect::Value(UNUSED),
    },
    Case {
        name: CaseName("a_dash_inside_a_rule_id_is_not_the_separator"),
        word: Word::Succeeds,
        input: || ("# ignore: p/r-x - a reason\nkey = 1\n", 2),
        expect: Expect::Value(UNUSED),
    },
    // No Written at all
    Case {
        name: CaseName("a_comment_that_is_not_an_ignore_is_not_read_as_one"),
        word: Word::Refuses,
        input: || ("# ignored: p/r - not the marker\nkey = 1\n", 2),
        expect: Expect::error(Refusal::NotAnIgnore),
    },
];

/// The comments of `text` as the shared `comment` relation gives those of a language whose comments open with `#` and run
/// to the end of their line: the texts here hold no `#` in a string.
fn comments_of(text: &str) -> Vec<Comment> {
    let mut found = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        if let Some(at) = line.find('#') {
            let body = line
                .get(at + 1..)
                .unwrap_or_default()
                .trim_end_matches(['\n', '\r']);
            found.push(comment(
                ByteSpan {
                    start: ByteOffset(start + at),
                    end: ByteOffset(start + at + 1 + body.len()),
                },
                Documentation::Plain,
                CommentText(body),
            ));
        }
        start += line.len();
    }
    found
}

fn subject((text, line): Input) -> Result<Value, Failure> {
    let comments = comments_of(text);
    let written = comments
        .iter()
        .any(|comment| matches!(&comment.kind, CommentKind::Line(body) if read(body).is_some()));
    if !written {
        return Err(Refusal::NotAnIgnore);
    }
    let source = lines(SourceText(text), &comments);
    let known = BTreeSet::from([RuleId("p/r".into())]);
    let read = ignores(SourceText(text), &source, &comments, &known);
    let reach = read.ignores.first().map(|ignore| ignore.reach);
    let place = if line == 0 {
        Place::Whole
    } else {
        let before = usize::try_from(line - 1).unwrap_or(0);
        let start = text
            .split_inclusive('\n')
            .take(before)
            .map(str::len)
            .sum::<usize>();
        let length = text
            .lines()
            .nth(before)
            .map_or(0, |row| row.trim_end().len());
        spanning(
            SourceText(text),
            ByteOffset(start),
            ByteOffset(start + length),
        )
        .map_or(Place::Whole, Place::At)
    };
    let found = vec![finding(
        crate::domain::findings::service::of_rule(RuleId("p/r".into())),
        RepoPath("f.toml".into()),
        place,
        "m".to_owned(),
        Help::Silent,
    )];
    let Sifted {
        kept,
        suppressed,
        problems,
    } = apply(found, SourceText(text), &source, read);
    Ok(Outcome {
        reach,
        kept: kept.len(),
        suppressed: suppressed.len(),
        problems: problems.len(),
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
