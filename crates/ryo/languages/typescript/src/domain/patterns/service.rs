//! The `patterns` service: free functions named by verb over a file's tree, with no IO and no ports. They read what the language's
//! library parsed of the regular-expression patterns a file carries: the properties it left on a carrier and on each pattern element,
//! and where each element stands under the carrier.

use super::constants::{BODY, MALFORMED, MALFORMED_FLAGS, MAX, MIN, PARSED};
use super::types::{PatternRole, Refusal, RefusedPart};
use crate::domain::syntax::constants::{
    PATTERN_ERROR, PATTERN_ERROR_END, PATTERN_ERROR_START, PATTERN_FLAGS, PATTERN_FLAGS_ERROR,
    PATTERN_VALUE,
};
use crate::domain::syntax::service::{kind, offset, parent, written};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::PatternNode;
use crate::domain::syntax::reading::{Interner, Syntax};
use ryo::languages::{self, Integer, NameText, Position, Symbol};

/// The flags the element at `at` had the pattern it carries read with; nothing when it carries no pattern.
#[must_use]
pub(crate) fn flags(tree: &impl Syntax, symbols: &mut impl Interner, at: Position) -> Option<Symbol> {
    tree.property_of(at, PATTERN_FLAGS)
        .map(|text| symbols.derived(tree.bytes(), text))
}

/// What the library refused of the carrier at `at`: its flags, its pattern, or nothing.
#[must_use]
fn refused<'t>(tree: &'t impl Syntax, at: Position) -> Option<Refusal<'t>> {
    if let Some(message) = tree.property_of(at, PATTERN_FLAGS_ERROR) {
        return Some(Refusal {
            part: RefusedPart::Flags,
            message,
        });
    }
    tree.property_of(at, PATTERN_ERROR).map(|message| Refusal {
        part: RefusedPart::Pattern,
        message,
    })
}

/// Whether the library parsed the pattern the element at `at` carries, refused it, or refused the flags it was given.
#[must_use]
pub(crate) fn outcome(tree: &impl Syntax, symbols: &mut impl Interner, at: Position) -> Symbol {
    let word = match refused(tree, at).map(|refused| refused.part) {
        None => PARSED,
        Some(RefusedPart::Pattern) => MALFORMED,
        Some(RefusedPart::Flags) => MALFORMED_FLAGS,
    };
    symbols.derived(tree.bytes(), NameText(word))
}

/// The library's message for what it refused of the carrier at `at`; no text when it refused nothing.
#[must_use]
pub(crate) fn error(tree: &impl Syntax, symbols: &mut impl Interner, at: Position) -> Symbol {
    refused(tree, at).map_or(languages::NO_SYMBOL, |refused| {
        symbols.derived(tree.bytes(), refused.message)
    })
}

/// Where the library refused what the carrier at `at` gave it, when it did and said where; the carrier's own extent otherwise.
#[must_use]
pub(crate) fn span(tree: &impl Syntax, at: Position) -> languages::Span {
    match (
        offset(tree, at, PATTERN_ERROR_START),
        offset(tree, at, PATTERN_ERROR_END),
    ) {
        (Some(start), Some(end)) => languages::Span { start, end },
        _ => languages::Span {
            start: tree.start_of(at),
            end: tree.end_of(at),
        },
    }
}

/// The element that carries the pattern the element at `at` belongs to: the nearest element above it that is not of the pattern.
#[must_use]
pub(crate) fn carrier(tree: &impl Syntax, at: Position) -> Position {
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
pub(crate) fn value(tree: &impl Syntax, at: Position) -> Integer {
    let Some(text) = tree.property_of(at, PATTERN_VALUE) else {
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
pub(crate) fn text(tree: &impl Syntax, symbols: &mut impl Interner, at: Position) -> Symbol {
    symbols.derived(tree.bytes(), written(tree, at))
}

/// Whether the element at `at` stands under another element of its pattern, and not directly under the carrier.
#[must_use]
pub(crate) fn nested(tree: &impl Syntax, at: Position) -> languages::Flag {
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
pub(crate) fn role(tree: &impl Syntax, at: Position) -> PatternRole {
    if kind(tree, parent(tree, at)) == Kind::Pattern(PatternNode::CharacterClassRange) {
        if tree.ordinal_of(at).0 == 0 {
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
pub(crate) fn field(tree: &impl Syntax, symbols: &mut impl Interner, at: Position) -> Symbol {
    let name = match role(tree, at) {
        PatternRole::Body => BODY,
        PatternRole::Min => MIN,
        PatternRole::Max => MAX,
    };
    symbols.derived(tree.bytes(), NameText(name))
}
