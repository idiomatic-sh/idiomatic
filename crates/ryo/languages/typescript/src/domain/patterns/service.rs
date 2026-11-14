//! The `patterns` service: free functions named by verb over a file's tree, with no IO and no ports. They read what the language's
//! library parsed of the regular-expression patterns a file carries: the properties it left on a carrier and on each pattern element,
//! and where each element stands under the carrier.

use super::constants::{BODY, MALFORMED, MAX, MIN, PARSED};
use super::types::PatternRole;
use crate::domain::syntax::constants::{PATTERN_ERROR, PATTERN_FLAGS, PATTERN_VALUE};
use crate::domain::syntax::service::{kind, parent, written};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::PatternNode;
use ryo::languages::{self, Integer, NameText, Position, Symbol, Symbols, Tree};

/// The flags the element at `at` had the pattern it carries read with; nothing when it carries no pattern.
#[must_use]
pub(crate) fn flags(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Option<Symbol> {
    languages::property_at(tree.elements, at, PATTERN_FLAGS)
        .map(|text| languages::intern_derived(symbols, tree.source, text))
}

/// Whether the library parsed the pattern the element at `at` carries, or refused it.
#[must_use]
pub(crate) fn outcome(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    let word = if languages::property_at(tree.elements, at, PATTERN_ERROR).is_some() {
        MALFORMED
    } else {
        PARSED
    };
    languages::intern_derived(symbols, tree.source, NameText(word))
}

/// The library's message for the pattern the element at `at` carries, when it refused it; no text when it parsed it.
#[must_use]
pub(crate) fn error(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    languages::property_at(tree.elements, at, PATTERN_ERROR).map_or(languages::NO_SYMBOL, |text| {
        languages::intern_derived(symbols, tree.source, text)
    })
}

/// The element that carries the pattern the element at `at` belongs to: the nearest element above it that is not of the pattern.
#[must_use]
pub(crate) fn carrier(tree: &Tree<'_>, at: Position) -> Position {
    let mut current = at;
    loop {
        let above = parent(tree, current);
        if above == current || !matches!(kind(tree, above), Kind::Pattern(_)) {
            return above;
        }
        current = above;
    }
}

/// The number the pattern element at `at` stands for: a character's code point, a reference's group index; 0 for any other.
#[must_use]
pub(crate) fn value(tree: &Tree<'_>, at: Position) -> Integer {
    let Some(text) = languages::property_at(tree.elements, at, PATTERN_VALUE) else {
        return Integer(0);
    };
    // The adapter writes the number in decimal; text that is not one was never written by it, and stands for no number.
    let Ok(number) = text.0.parse::<i64>() else {
        return Integer(0);
    };
    Integer(number)
}

/// The pattern element at `at` as the pattern spells it.
#[must_use]
pub(crate) fn text(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    languages::intern_derived(symbols, tree.source, written(tree, at))
}

/// Whether the element at `at` stands under another element of its pattern, and not directly under the carrier.
#[must_use]
pub(crate) fn nested(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let above = parent(tree, at);
    if above != at && matches!(kind(tree, above), Kind::Pattern(_)) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The role of the pattern element at `at` under its parent: the ends of a range are its low and high end, every other child is
/// what its parent holds.
#[must_use]
pub(crate) fn role(tree: &Tree<'_>, at: Position) -> PatternRole {
    if kind(tree, parent(tree, at)) == Kind::Pattern(PatternNode::CharacterClassRange) {
        if languages::ordinal_at(tree.elements, at).0 == 0 {
            PatternRole::Min
        } else {
            PatternRole::Max
        }
    } else {
        PatternRole::Body
    }
}

/// The name of the role of the pattern element at `at`, as the `field` column holds it.
#[must_use]
pub(crate) fn field(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    let name = match role(tree, at) {
        PatternRole::Body => BODY,
        PatternRole::Min => MIN,
        PatternRole::Max => MAX,
    };
    languages::intern_derived(symbols, tree.source, NameText(name))
}
