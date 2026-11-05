//! The `prose` service's cases: the kinds a run reads, and how each adds to its text.

use super::service::inline_of;
use super::types::Inline;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The kind of an inline element.
type Input = Kind;
/// What it adds to a run.
type Value = Inline;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_adds_its_words"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::Text),
        expect: Expect::Value(Inline::Words),
    },
    Case {
        name: CaseName("code_is_masked"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::InlineCode),
        expect: Expect::Value(Inline::Mask),
    },
    Case {
        name: CaseName("raw_html_is_masked"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::Html),
        expect: Expect::Value(Inline::Mask),
    },
    Case {
        name: CaseName("a_break_is_a_space"),
        word: Word::Succeeds,
        input: || Kind::Token(Token::LineEnding),
        expect: Expect::Value(Inline::Mask),
    },
    Case {
        name: CaseName("emphasis_adds_what_it_holds"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::Emphasis),
        expect: Expect::Value(Inline::Holds),
    },
    Case {
        name: CaseName("a_link_adds_what_it_holds"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::Link),
        expect: Expect::Value(Inline::Holds),
    },
    Case {
        name: CaseName("a_link_written_with_a_label_adds_what_it_holds"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::LinkReference),
        expect: Expect::Value(Inline::Holds),
    },
    Case {
        name: CaseName("an_image_written_with_a_label_adds_what_it_holds"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::ImageReference),
        expect: Expect::Value(Inline::Holds),
    },
    Case {
        name: CaseName("a_block_in_a_run_adds_nothing"),
        word: Word::Succeeds,
        input: || Kind::Node(MarkdownNode::List),
        expect: Expect::Value(Inline::Nothing),
    },
];

fn subject(of: Input) -> Value {
    inline_of(of)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
