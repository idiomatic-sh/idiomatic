//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and moves
//! about a file's syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{
    BLOCK_CLOSE, BLOCK_OPEN, EMPTY_BLOCK, INNER_BLOCK_DOC, INNER_LINE_DOC, LINE_OPEN,
    OUTER_BLOCK_DOC, OUTER_LINE_DOC, PLAIN_BLOCK_PREFIX, PLAIN_LINE_PREFIX, SPACE,
};
use super::types::{Containment, Gap, Kind, Leaf, Offspring, Rootness, Typeness};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub(crate) Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Other));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// Where the element at `at` starts.
#[must_use]
pub(crate) fn start(tree: &Tree<'_>, at: languages::Position) -> languages::Offset {
    languages::start_at(tree.elements, at)
}

/// Where the element at `at` ends.
#[must_use]
pub(crate) fn end(tree: &Tree<'_>, at: languages::Position) -> languages::Offset {
    languages::end_at(tree.elements, at)
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: languages::Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The text of the token at `at`, as the name it was interned as.
#[must_use]
pub(crate) fn text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    languages::text_at(tree.elements, at)
}

/// The content of a comment without its line, block or documentation delimiters.
pub(crate) fn comment_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let text = comment_source(tree, at).0;
    let inner = if text.starts_with(PLAIN_LINE_PREFIX) {
        text.strip_prefix(LINE_OPEN).unwrap_or(text)
    } else if let Some(rest) = text
        .strip_prefix(OUTER_LINE_DOC)
        .or_else(|| text.strip_prefix(INNER_LINE_DOC))
    {
        rest
    } else if let Some(rest) = text.strip_prefix(LINE_OPEN) {
        rest
    } else {
        let rest = if text.starts_with(PLAIN_BLOCK_PREFIX) || text == EMPTY_BLOCK {
            text.strip_prefix(BLOCK_OPEN)
        } else {
            text.strip_prefix(OUTER_BLOCK_DOC)
                .or_else(|| text.strip_prefix(INNER_BLOCK_DOC))
                .or_else(|| text.strip_prefix(BLOCK_OPEN))
        }
        .unwrap_or(text);
        rest.strip_suffix(BLOCK_CLOSE).unwrap_or(rest)
    };
    interned(tree, symbols, languages::NameText(inner))
}

/// The comment at `at` as the file writes it, delimiters included.
fn comment_source<'a>(tree: &Tree<'a>, at: languages::Position) -> languages::NameText<'a> {
    let from = usize::try_from(start(tree, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(end(tree, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .unwrap_or_default(),
    )
}

/// Whether the comment at `at` is documentation, as rustdoc reads it: `///` and `/**` document what follows, `//!` and `/*!` what
/// encloses them; `////`, `/***` and the empty `/**/` are plain comments.
#[must_use]
pub(crate) fn documentation(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    let text = comment_source(tree, at).0;
    let plain = text.starts_with(PLAIN_LINE_PREFIX)
        || text.starts_with(PLAIN_BLOCK_PREFIX)
        || text == EMPTY_BLOCK;
    let documents = [
        OUTER_LINE_DOC,
        INNER_LINE_DOC,
        OUTER_BLOCK_DOC,
        INNER_BLOCK_DOC,
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix));
    if documents && !plain {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// The last element under the element at `at`: its descendants are the elements right after it, up to this one.
#[must_use]
pub(crate) fn last_descendant(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::last_at(tree.elements, at)
}

/// Whether `at` is the root of the tree.
#[must_use]
pub(crate) fn is_root(tree: &Tree<'_>, at: languages::Position) -> Rootness {
    if parent(tree, at) == at {
        Rootness::Root
    } else {
        Rootness::Inner
    }
}

/// The first child of the element at `of` that is of `wanted` kind.
#[must_use]
pub(crate) fn child(
    tree: &Tree<'_>,
    of: languages::Position,
    wanted: Kind,
) -> Option<languages::Position> {
    languages::children(tree.elements, of).find(|each| kind(tree, *each) == wanted)
}

/// The first child of the element at `of` that is a node: not a token, so neither trivia nor punctuation.
#[must_use]
pub(crate) fn first_node(tree: &Tree<'_>, of: languages::Position) -> Option<languages::Position> {
    languages::children(tree.elements, of).find(|each| is_token(tree, *each) == Leaf::Node)
}

/// The first child of the element at `of` that is a node and not an attribute: a pattern, a type, an expression.
#[must_use]
pub(crate) fn first_body(tree: &Tree<'_>, of: languages::Position) -> Option<languages::Position> {
    languages::children(tree.elements, of).find(|each| {
        (is_token(tree, *each) == Leaf::Node) && kind(tree, *each) != Kind::Node(Node::Attr)
    })
}

/// The first node after the token of `marker` kind that is a child of `of`: the type after a
/// colon, the value after an equals sign.
#[must_use]
pub(crate) fn node_after(
    tree: &Tree<'_>,
    of: languages::Position,
    marker: Kind,
) -> Option<languages::Position> {
    let mut after = false;
    for each in languages::children(tree.elements, of) {
        if after && (is_token(tree, each) == Leaf::Node) {
            return Some(each);
        }
        after |= (is_token(tree, each) == Leaf::Token) && kind(tree, each) == marker;
    }
    None
}

/// The text of the name a `NAME` or `NAME_REF` node at `of` holds: its first token.
#[must_use]
pub(crate) fn name_text(tree: &Tree<'_>, of: languages::Position) -> Option<languages::Symbol> {
    languages::children(tree.elements, of)
        .find(|each| is_token(tree, *each) == Leaf::Token)
        .map(|each| text(tree, each))
}

/// The ancestors of the element at `at`, nearest first, the root last; `at` itself is not one.
pub(crate) fn ancestors<'a>(
    tree: &'a Tree<'a>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let mut next = at;
    std::iter::from_fn(move || {
        if is_root(tree, next) == Rootness::Root {
            return None;
        }
        next = parent(tree, next);
        Some(next)
    })
}

/// The name of the text between `from` and `to` of the file, which is not a token of its own.
pub(crate) fn written(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    from: languages::Offset,
    to: languages::Offset,
) -> languages::Symbol {
    let bytes = tree
        .source
        .0
        .get(
            usize::try_from(from.0).unwrap_or(usize::MAX)
                ..usize::try_from(to.0).unwrap_or(usize::MAX),
        )
        .unwrap_or(&[]);
    languages::intern_derived(
        symbols,
        tree.source,
        languages::NameText(std::str::from_utf8(bytes).unwrap_or_default()),
    )
}

/// Pushes the token at `at` onto the name being made: white space and comments as one space between two tokens, any other token as
/// written.
pub(crate) fn push_token(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
    gap: &mut Gap,
) {
    if matches!(
        kind(tree, at),
        Kind::Token(Token::Whitespace | Token::Comment)
    ) {
        if *gap == Gap::Joined {
            *gap = Gap::Space;
        }
        return;
    }
    if *gap == Gap::Space {
        languages::push_text(symbols, languages::NameText(SPACE));
    }
    *gap = Gap::Joined;
    languages::push_span(symbols, tree.source, start(tree, at), end(tree, at));
}

/// Pushes every token under `at` onto the name being made, as `push_token` does.
fn push_tokens(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
    gap: &mut Gap,
) {
    for each in languages::children(tree.elements, at) {
        if is_token(tree, each) == Leaf::Node {
            push_tokens(tree, symbols, each, gap);
        } else {
            push_token(tree, symbols, each, gap);
        }
    }
}

/// The name of the text of the element at `at` as written, the white space and comments between two of its tokens one space.
pub(crate) fn collapsed(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let begun = languages::begin_name(symbols);
    push_tokens(tree, symbols, at, &mut Gap::Start);
    languages::end_name(symbols, tree.source, begun)
}

/// Whether an ancestor of the element at `at` is of `wanted` kind.
#[must_use]
pub(crate) fn is_inside(tree: &Tree<'_>, at: languages::Position, wanted: Kind) -> Containment {
    if ancestors(tree, at).any(|each| kind(tree, each) == wanted) {
        Containment::Inside
    } else {
        Containment::Outside
    }
}

/// Whether the element at `at` has a child of `wanted` kind.
#[must_use]
pub(crate) fn has_child(tree: &Tree<'_>, at: languages::Position, wanted: Kind) -> Offspring {
    if child(tree, at, wanted).is_some() {
        Offspring::Has
    } else {
        Offspring::Lacks
    }
}

/// The place of the element at `at` among its parent's children.
#[must_use]
pub(crate) fn ordinal(tree: &Tree<'_>, at: languages::Position) -> languages::ChildOrdinal {
    languages::ordinal_at(tree.elements, at)
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        text(tree, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// The name a `NAME` child of the element at `at` holds: its first token.
#[must_use]
pub(crate) fn named(tree: &Tree<'_>, at: languages::Position) -> Option<languages::Symbol> {
    languages::first_token_text(tree.elements, child(tree, at, Kind::Node(Node::Name))?)
}

/// The name `text` as a name made after the file was read.
pub(crate) fn interned(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    text: languages::NameText<'_>,
) -> languages::Symbol {
    languages::intern_derived(symbols, tree.source, text)
}

/// The place of the `number`-th thing the parser reported about the file, as a node id of the file: a report is known by its number.
#[must_use]
pub(crate) fn diagnostic_id(tree: &Tree<'_>, number: languages::ChildOrdinal) -> languages::NodeId {
    languages::diagnostic_id(tree.file, number)
}

/// What the parser said at one place, as a name.
pub(crate) fn diagnostic_message(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    diagnostic: &languages::Diagnostic<'_>,
) -> languages::Symbol {
    interned(
        tree,
        symbols,
        languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()),
    )
}

/// Where the parser said it.
#[must_use]
pub(crate) fn diagnostic_span(diagnostic: &languages::Diagnostic<'_>) -> languages::Span {
    languages::Span {
        start: diagnostic.start,
        end: diagnostic.end,
    }
}

/// Whether the element at `at` is a type: a path, a reference, a tuple, a slice, an `impl Trait`.
/// The `-> T` wrapper of a function result is not one: its type is its child.
#[must_use]
pub(crate) fn is_type(tree: &Tree<'_>, at: languages::Position) -> Typeness {
    if matches!(
        kind(tree, at),
        Kind::Node(
            Node::ArrayType
                | Node::DynTraitType
                | Node::FnPtrType
                | Node::ForType
                | Node::ImplTraitType
                | Node::InferType
                | Node::MacroType
                | Node::NeverType
                | Node::ParenType
                | Node::PathType
                | Node::PatternType
                | Node::PtrType
                | Node::RefType
                | Node::SliceType
                | Node::TupleType
        )
    ) {
        Typeness::Type
    } else {
        Typeness::NotType
    }
}
