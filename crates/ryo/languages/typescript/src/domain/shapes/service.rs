//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is TypeScript's; interning them is the kernel's.

use crate::domain::fields::constants::PARENTHESES;

use crate::domain::names::service::text;
use crate::domain::syntax::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use crate::domain::syntax::reading::{Interner, Syntax};
use crate::domain::syntax::service::{is_token, kind};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, NO_FIELD, Position, ShapeKey, ShapeWord, child_class, push_shape_word};

/// The key of a node of the numbered view, built in a retained key index.
pub(crate) struct Numbered<'i> {
    /// The index the key is built in.
    pub(crate) index: &'i mut languages::RetainedKeyIndex<Position, ShapeWord>,
    /// Whether every word was kept.
    pub(crate) whole: bool,
}

impl Sink for Numbered<'_> {
    fn push(&mut self, word: ShapeWord) {
        if languages::push_key_word(self.index, word).is_none() {
            self.whole = false;
        }
    }
    fn class(&self, child: Position) -> Option<Position> {
        self.index.classes.get(usize::from(child)).copied().flatten()
    }
}

/// The word a token adds to its node's key; none for a comment, a gap, a parenthesis, or the digits of a numeric literal whose
/// value the node already holds.
fn token_word(
    tree: &impl Syntax,
    at: Position,
    symbols: &mut impl Interner,
    numeric: Option<languages::NameText<'_>>,
) -> Option<ShapeWord> {
    match kind(tree, at) {
        Kind::Token(Token::Line | Token::Block)
        | Kind::Extra(_)
        | Kind::Node(_)
        | Kind::Pattern(_) => None,
        Kind::Token(Token::Numeric) if numeric.is_some() => None,
        Kind::Token(Token::Punctuator)
            if PARENTHESES
                .iter()
                .any(|marker| marker.0 == text(tree, at).0) =>
        {
            None
        }
        Kind::Token(_) => Some(ShapeWord::Token(
            tree.kind_bits(at),
            match tree.decoded_of(at) {
                Some(decoded) => symbols.derived(tree.bytes(), decoded),
                None => tree.spelling(symbols, at),
            },
        )),
    }
}


/// Where the words of a key go, and the classes the children already have.
pub(crate) trait Sink {
    /// Appends a word to the key.
    fn push(&mut self, word: ShapeWord);
    /// The class a child already has.
    fn class(&self, child: Position) -> Option<Position>;
}

impl Sink for ShapeKey<'_> {
    fn push(&mut self, word: ShapeWord) {
        push_shape_word(self, word);
    }
    fn class(&self, child: Position) -> Option<Position> {
        child_class(self, child)
    }
}

/// The words of the key of the node at `at`, each handed to `push`; `class` answers the class a child already has.
pub(crate) fn words(
    tree: &impl Syntax,
    at: Position,
    symbols: &mut impl Interner,
    key: &mut impl Sink,
) -> Option<()> {
    key.push(ShapeWord::Kind(tree.kind_bits(at)));
    for property in [NUMERIC_KIND, NUMERIC_VALUE] {
        if let Some(value) = tree.property_of(at, property) {
            let value = symbols.derived(tree.bytes(), value);
            key.push(ShapeWord::Property(property, value));
        }
    }
    let numeric = tree.property_of(at, NUMERIC_VALUE);
    for child in tree.children_of(at) {
        let word = match is_token(tree, child) {
            Leaf::Node => ShapeWord::Child(NO_FIELD, key.class(child)?),
            Leaf::Token => match token_word(tree, child, symbols, numeric) {
                Some(word) => word,
                None => continue,
            },
        };
        key.push(word);
    }
    Some(())
}

