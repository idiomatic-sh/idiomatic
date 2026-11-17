//! What the `comments` service reads: each row a comment's content as the shared `comment` relation gives it, and what
//! it is read as.

use super::service::comment;
use super::types::{ByteSpan, CommentKind, CommentText, Documentation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::ByteOffset;

/// A comment's content, without its delimiters.
type Input = &'static str;
/// What it was read as: `line:<content>` or `other`.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_comment_of_one_line_may_hold_an_ignore"),
        word: Word::Succeeds,
        input: || " ignore: p/rule - why",
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_comment_of_several_lines_holds_none"),
        word: Word::Succeeds,
        input: || " first\n ignore: p/rule - why\n",
        expect: Expect::Value(String::new()),
    },
];

/// What a row's comment is read as when the service is right.
fn expected(text: Input) -> String {
    if text.contains('\n') {
        "other".to_owned()
    } else {
        format!("line:{text}")
    }
}

fn subject(text: Input) -> Value {
    let span = ByteSpan {
        start: ByteOffset(0),
        end: ByteOffset(text.len()),
    };
    let said = match comment(span, Documentation::Plain, CommentText(text)).kind {
        CommentKind::Line(body) => format!("line:{}", body.0),
        CommentKind::Other => "other".to_owned(),
    };
    if said == expected(text) {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
