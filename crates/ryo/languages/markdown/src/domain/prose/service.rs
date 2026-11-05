//! The `prose` service: free functions named by verb that read the text of a run of a document, with no IO and no ports. A run is a
//! paragraph, the title of a heading, a cell of a table, or the text an item of a list holds itself. Its markup is joined away, its code,
//! math and raw HTML are masked by a space, a break is a space, and the white space between words is one space.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Tree};

use super::constants::{CODE_FENCE, WORD_SPACE};
use super::types::{Code, Inline, Spoken, WordGap, Wording};
use crate::domain::syntax::service::{kind, written};
use crate::domain::syntax::types::Kind;

/// What an inline element of the kind `of` adds to a run.
#[must_use]
pub(crate) fn inline_of(of: Kind) -> Inline {
    if of == Kind::Node(MarkdownNode::Text) {
        Inline::Words
    } else if matches!(
        of,
        Kind::Token(Token::LineEnding | Token::HtmlFlowData)
            | Kind::Node(MarkdownNode::Break | MarkdownNode::InlineCode | MarkdownNode::Html)
            | Kind::Extra(Extra::InlineMath)
    ) {
        Inline::Mask
    } else if matches!(
        of,
        Kind::Node(
            MarkdownNode::Emphasis
                | MarkdownNode::Strong
                | MarkdownNode::Delete
                | MarkdownNode::Link
                | MarkdownNode::LinkReference
                | MarkdownNode::Image
                | MarkdownNode::ImageReference
        ) | Kind::Extra(Extra::Superscript | Extra::Subscript)
    ) {
        Inline::Holds
    } else {
        Inline::Nothing
    }
}

/// Whether the run at `at` has a word a reader reads.
#[must_use]
pub(crate) fn has_words(tree: &Tree<'_>, at: languages::Position) -> Wording {
    let spoken =
        languages::children(tree.elements, at).any(|child| match inline_of(kind(tree, child)) {
            Inline::Words => !written(tree, child).0.trim().is_empty(),
            Inline::Holds => has_words(tree, child) == Wording::Words,
            Inline::Mask | Inline::Nothing => false,
        });
    if spoken {
        Wording::Words
    } else {
        Wording::Silent
    }
}

/// Writes the text of what the element at `at` holds into the name being made: each word, with one space before it when a word came
/// before; a mask is a space between the words around it.
fn gather(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
    spoken: &mut Spoken,
    code: Code,
) {
    for child in languages::children(tree.elements, at) {
        let of = kind(tree, child);
        let inline = if code == Code::Read && of == Kind::Node(MarkdownNode::InlineCode) {
            Inline::Words
        } else {
            inline_of(of)
        };
        match inline {
            Inline::Words => {
                let text = written(tree, child).0;
                // The code a code span holds: its text inside its backtick strings, one space of padding on each side
                // left out when both are there and the code is not all spaces (CommonMark 6.1).
                let text = if of == Kind::Node(MarkdownNode::InlineCode) {
                    let inner = text.trim_matches(CODE_FENCE);
                    inner
                        .strip_prefix(' ')
                        .and_then(|rest| rest.strip_suffix(' '))
                        .filter(|rest| !rest.trim().is_empty())
                        .unwrap_or(inner)
                } else {
                    text
                };
                if text.starts_with(char::is_whitespace) {
                    spoken.gap = WordGap::Open;
                }
                for (place, word) in text.split_whitespace().enumerate() {
                    // Words are one space apart when something came between them: white space, a break, a mask.
                    if spoken.any == Wording::Words && (place > 0 || spoken.gap == WordGap::Open) {
                        languages::push_text(symbols, WORD_SPACE);
                    }
                    languages::push_text(symbols, languages::NameText(word));
                    spoken.any = Wording::Words;
                    spoken.gap = WordGap::Closed;
                }
                if text.ends_with(char::is_whitespace) {
                    spoken.gap = WordGap::Open;
                }
            }
            Inline::Mask => spoken.gap = WordGap::Open,
            Inline::Holds => gather(tree, symbols, child, spoken, code),
            Inline::Nothing => {}
        }
    }
}

/// The text of the run at `at`, as a name: what a reader reads, with one space between words and none at the ends.
pub(crate) fn run_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    let mut spoken = Spoken {
        any: Wording::Silent,
        gap: WordGap::Closed,
    };
    gather(tree, symbols, at, &mut spoken, Code::Masked);
    languages::end_name(symbols, tree.source, from)
}

/// The title of the section the heading at `at` opens, as a name: its words, and the code of its code spans read as words,
/// as a reader of the title reads it and a link to it spells it.
pub(crate) fn section_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    let mut spoken = Spoken {
        any: Wording::Silent,
        gap: WordGap::Closed,
    };
    gather(tree, symbols, at, &mut spoken, Code::Read);
    languages::end_name(symbols, tree.source, from)
}
