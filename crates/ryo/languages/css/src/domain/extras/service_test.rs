//! The `extras` service's cases: a comment is read, the rest only kept.

use super::service::content;
use super::types::Content;
use super::types::Extra;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// An extra.
type Input = Extra;
/// What it is to the readers.
type Value = Content;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_comment_is_read"),
        word: Word::Succeeds,
        input: || Extra::Comment,
        expect: Expect::Value(Content::Read),
    },
    Case {
        name: CaseName("a_byte_order_mark_is_kept"),
        word: Word::Succeeds,
        input: || Extra::ByteOrderMark,
        expect: Expect::Value(Content::Kept),
    },
];

fn subject(extra: Input) -> Value {
    content(extra)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |extra| {
        Ok::<Value, Failure>(subject(extra))
    });
}
