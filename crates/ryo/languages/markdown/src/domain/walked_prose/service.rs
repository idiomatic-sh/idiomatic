//! The `walked_prose` service: free functions that read the text of a run of a walked document, and the code a block or a span holds,
//! with no IO and no ports. A run is a paragraph, the title of a heading, a cell of a table, or the text an item of a list holds itself.
//! Its markup is joined away, its code, math and raw HTML are masked by a space, a break is a space, and the white space between
//! words is one space.

use crate::domain::file_view::service::{children, decoded, kind, written};
use crate::domain::file_view::types::Syntax;
use crate::domain::nodes::types::Node;
use crate::domain::prose::constants::{CODE_FENCE, WORD_SPACE};
use crate::domain::prose::service::inline_of;
use crate::domain::prose::types::{Code, Inline, Spoken, WordGap, Wording};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::Position;

/// Whether the run at `at` holds a word.
#[must_use]
pub(crate) fn has_words(syntax: &Syntax<'_>, at: Position) -> Wording {
    let spoken = children(syntax, at).any(|child| match inline_of(kind(syntax, child)) {
        Inline::Words => !written(syntax, child).trim().is_empty(),
        Inline::Holds => has_words(syntax, child) == Wording::Words,
        Inline::Mask | Inline::Nothing => false,
    });
    if spoken {
        Wording::Words
    } else {
        Wording::Silent
    }
}

/// Appends the words of the run at `at` to `out`, one space apart.
fn gather(syntax: &Syntax<'_>, at: Position, spoken: &mut Spoken, code: Code, out: &mut String) {
    for child in children(syntax, at) {
        let of = kind(syntax, child);
        let inline = if code == Code::Read && of == Kind::Node(Node::InlineCode) {
            Inline::Words
        } else {
            inline_of(of)
        };
        match inline {
            Inline::Words => {
                let text = written(syntax, child);
                // The code a code span holds: its text inside its backtick strings, one space of padding on each side
                // left out when both are there and the code is not all spaces (CommonMark 6.1).
                let text = if of == Kind::Node(Node::InlineCode) {
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
                        out.push_str(WORD_SPACE.0);
                    }
                    out.push_str(word);
                    spoken.any = Wording::Words;
                    spoken.gap = WordGap::Closed;
                }
                if text.ends_with(char::is_whitespace) {
                    spoken.gap = WordGap::Open;
                }
            }
            Inline::Mask => spoken.gap = WordGap::Open,
            Inline::Holds => gather(syntax, child, spoken, code, out),
            Inline::Nothing => {}
        }
    }
}

/// The words of the run at `at`, code masked, into `out`.
pub(crate) fn run_text(syntax: &Syntax<'_>, at: Position, out: &mut String) {
    out.clear();
    let mut spoken = Spoken {
        any: Wording::Silent,
        gap: WordGap::Closed,
    };
    gather(syntax, at, &mut spoken, Code::Masked, out);
}

/// The words of the heading at `at`, code read, into `out`.
pub(crate) fn section_text(syntax: &Syntax<'_>, at: Position, out: &mut String) {
    out.clear();
    let mut spoken = Spoken {
        any: Wording::Silent,
        gap: WordGap::Closed,
    };
    gather(syntax, at, &mut spoken, Code::Read, out);
}

/// The code the block or span at `at` holds, into `out`: a span's own code, a block's code values joined.
pub(crate) fn code_text(syntax: &Syntax<'_>, at: Position, out: &mut String) {
    out.clear();
    if kind(syntax, at) == Kind::Node(Node::InlineCode) {
        if let Some(text) = decoded(syntax, at) {
            out.push_str(text);
        }
    } else {
        for child in children(syntax, at) {
            if kind(syntax, child) == Kind::Token(Token::CodeFlowValue)
                && let Some(text) = decoded(syntax, child)
            {
                out.push_str(text);
            }
        }
    }
}
