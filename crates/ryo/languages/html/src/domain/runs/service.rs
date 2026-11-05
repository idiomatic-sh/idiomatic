//! The `runs` service: free functions named by verb over the runs of text of an HTML tree, with no IO and no ports. A run is the
//! text a block element holds, its inline markup joined: what a reader reads as one stretch of prose.

use ryo::languages::{self, Tree};

use super::types::Gap;
use crate::domain::nodes::types::HtmlNode;
use crate::domain::syntax::constants::{BREAK, CODE, INLINE, SCRIPT, STYLE, TEMPLATE, WORD_SPACE};
use crate::domain::syntax::service::{field, kind, owner, parent, section};
use crate::domain::syntax::types::{Flow, Kind, Property, Readability, Section, Wording};

/// The section that holds the run of the element at `at`: the element itself when it is one.
#[must_use]
pub(crate) fn run_owner(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    if section(tree, at) == Section::Section {
        at
    } else {
        owner(tree, at)
    }
}
/// Human-readable text excludes script and style data and template contents: the element at `at` and what it sits in.
#[must_use]
pub(crate) fn readable(tree: &Tree<'_>, at: languages::Position) -> Readability {
    let mut cursor = at;
    loop {
        if matches!(
            field(tree, cursor, Property::Name).0,
            SCRIPT | STYLE | TEMPLATE
        ) {
            return Readability::Excluded;
        }
        if cursor == parent(tree, cursor) {
            return Readability::Prose;
        }
        cursor = parent(tree, cursor);
    }
}

/// Whether the element at `at` is part of the run around it or holds a run of its own.
#[must_use]
pub(crate) fn flow(tree: &Tree<'_>, at: languages::Position) -> Flow {
    if INLINE.contains(&field(tree, at, Property::Name).0) {
        Flow::Inline
    } else {
        Flow::Block
    }
}

/// Whether the element at `at` holds a run of its own with a word a reader reads: a block element that is read as prose and holds
/// text, in itself or in the inline elements it holds, besides code.
#[must_use]
pub(crate) fn run_words(tree: &Tree<'_>, at: languages::Position) -> Wording {
    if kind(tree, at) != Kind::Node(HtmlNode::Element)
        || flow(tree, at) == Flow::Inline
        || readable(tree, at) == Readability::Excluded
    {
        return Wording::Silent;
    }
    let mut any = Wording::Silent;
    gather(tree, None, at, &mut any, &mut Gap::Closed);
    any
}

/// The text of the run the element at `at` holds, as a name: its inline markup joined, its code, its breaks and the blocks it holds
/// each a space, and the white space between words one space.
pub(crate) fn run_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    let mut any = Wording::Silent;
    gather(tree, Some(symbols), at, &mut any, &mut Gap::Closed);
    languages::end_name(symbols, tree.source, from)
}

/// Walks the text of the run at `at`, writing each word into the name being made when `symbols` is given, and noting in `any` that
/// a word was read.
fn gather(
    tree: &Tree<'_>,
    mut symbols: Option<&mut languages::Symbols>,
    at: languages::Position,
    any: &mut Wording,
    gap: &mut Gap,
) {
    for child in languages::children(tree.elements, at) {
        match kind(tree, child) {
            Kind::Node(HtmlNode::Text) => {
                let text = field(tree, child, Property::Value).0;
                if text.starts_with(char::is_whitespace) {
                    *gap = Gap::Open;
                }
                let mut words = text.split_whitespace().peekable();
                while let Some(word) = words.next() {
                    if let Some(symbols) = symbols.as_deref_mut() {
                        if *gap == Gap::Open && *any == Wording::Words {
                            languages::push_text(symbols, WORD_SPACE);
                        }
                        languages::push_text(symbols, languages::NameText(word));
                    }
                    *any = Wording::Words;
                    *gap = if words.peek().is_some() || text.ends_with(char::is_whitespace) {
                        Gap::Open
                    } else {
                        Gap::Closed
                    };
                }
            }
            Kind::Node(HtmlNode::Element) => {
                let name = field(tree, child, Property::Name).0;
                if name == CODE || name == BREAK || flow(tree, child) == Flow::Block {
                    *gap = Gap::Open;
                } else {
                    match readable(tree, child) {
                        Readability::Prose => gather(tree, symbols.as_deref_mut(), child, any, gap),
                        Readability::Excluded => {}
                    }
                }
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}
