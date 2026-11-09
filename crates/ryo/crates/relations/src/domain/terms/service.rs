//! The `terms` service: free functions named by verb over the closed sets. A word is a name found
//! by its number alone, so a row holds it without a file having written it down.

#![allow(
    clippy::as_conversions,
    reason = "a closed set is a field-less enum with a u8 representation: its number is the cast"
)]

use super::constants::{
    BLOCK_SIZE, DECLARATION_OWNER, DEPENDENCY_OWNER, MAPPING_OWNER, ORIGIN_OWNER, PARAMETER_OWNER,
    TERM_BASE, UNKNOWN_OWNER, VALUE_OWNER,
};
use super::types::{Term, UnknownReason, WordBlock, Wordness};
use crate::domain::declaration_kind::types::DeclarationKind;
use crate::domain::dependency_kind::types::DependencyKind;
use crate::domain::dependency_origin::types::DependencyOrigin;
use crate::domain::mapping_kind::types::MappingKind;
use crate::domain::parameter_kind::types::ParameterKind;
use crate::domain::terms::types::ChildOrdinal;
use crate::domain::terms::types::NameText;
use crate::domain::terms::types::Symbol;
use crate::domain::value_kind::types::ValueKind;
use strum::IntoEnumIterator;

/// The first number of a block.
const fn start_of(block: WordBlock) -> Symbol {
    Symbol(
        TERM_BASE
            + BLOCK_SIZE
                * match block {
                    WordBlock::Declaration => 0,
                    WordBlock::Parameter => 1,
                    WordBlock::Value => 2,
                    WordBlock::Mapping => 3,
                    WordBlock::Dependency => 4,
                    WordBlock::Unknown => 5,
                    WordBlock::Origin => 6,
                },
    )
}

/// Implements `Term` for a set of terms: its block of numbers and the text of each.
macro_rules! term {
    ($set:ty, $block:expr) => {
        impl Term for $set {
            const BLOCK: WordBlock = $block;

            fn number(self) -> ChildOrdinal {
                ChildOrdinal(u32::from(self as u8))
            }

            fn text(self) -> NameText<'static> {
                NameText(self.into())
            }
        }
    };
}

term!(DeclarationKind, WordBlock::Declaration);
term!(ParameterKind, WordBlock::Parameter);
term!(ValueKind, WordBlock::Value);
term!(MappingKind, WordBlock::Mapping);
term!(DependencyKind, WordBlock::Dependency);
term!(DependencyOrigin, WordBlock::Origin);

/// The name a word is: the same word is the same name in every file.
#[must_use]
pub fn term_symbol<T>(term: T) -> Symbol
where
    T: Term,
{
    Symbol(start_of(T::BLOCK).0 + term.number().0)
}

/// Visit every closed-set symbol that spells `text`. Several vocabularies may
/// give different IDs to the same word, so callers must not assume one hit.
pub fn symbols_with_text(text: NameText<'_>, mut visit: impl FnMut(Symbol)) {
    macro_rules! visit_set {
        ($kind:ty) => {
            for term in <$kind>::iter() {
                if term.text() == text {
                    visit(term_symbol(term));
                }
            }
        };
    }
    visit_set!(DeclarationKind);
    visit_set!(ParameterKind);
    visit_set!(ValueKind);
    visit_set!(MappingKind);
    visit_set!(DependencyKind);
    visit_set!(DependencyOrigin);
}

/// The value a column holds when the language cannot resolve it statically, with the reason it declares for that: the same in every file.
#[must_use]
pub const fn unknown_symbol(reason: UnknownReason) -> Symbol {
    Symbol(start_of(WordBlock::Unknown).0 + reason.0 as u32)
}

/// The reason an unknown value was written for, when `symbol` is one.
#[must_use]
pub fn unknown_reason(symbol: Symbol) -> Option<UnknownReason> {
    let within = symbol.0.checked_sub(start_of(WordBlock::Unknown).0)?;
    // A symbol past the block of unknown reasons is no reason, which is the question asked.
    match u8::try_from(within) {
        Ok(number) => Some(UnknownReason(number)),
        Err(_) => None,
    }
}

/// The words of a closed set, in the order of their numbers.
#[must_use]
pub fn words_of(block: WordBlock) -> Vec<NameText<'static>> {
    (0..=u8::MAX)
        .map_while(|number| match block {
            WordBlock::Declaration => DeclarationKind::from_repr(number).map(Term::text),
            WordBlock::Parameter => ParameterKind::from_repr(number).map(Term::text),
            WordBlock::Value => ValueKind::from_repr(number).map(Term::text),
            WordBlock::Mapping => MappingKind::from_repr(number).map(Term::text),
            WordBlock::Dependency => DependencyKind::from_repr(number).map(Term::text),
            WordBlock::Origin => DependencyOrigin::from_repr(number).map(Term::text),
            WordBlock::Unknown => None,
        })
        .collect()
}

/// What the words of a closed set are the words of, as the catalog's listings name it: the relation whose column holds them.
#[must_use]
pub const fn owner_of(block: WordBlock) -> NameText<'static> {
    match block {
        WordBlock::Declaration => DECLARATION_OWNER,
        WordBlock::Parameter => PARAMETER_OWNER,
        WordBlock::Value => VALUE_OWNER,
        WordBlock::Mapping => MAPPING_OWNER,
        WordBlock::Dependency => DEPENDENCY_OWNER,
        WordBlock::Unknown => UNKNOWN_OWNER,
        WordBlock::Origin => ORIGIN_OWNER,
    }
}

/// Whether `symbol` is a word, not a name a file wrote.
#[must_use]
pub fn is_term(symbol: Symbol) -> Wordness {
    if symbol.0 >= TERM_BASE {
        Wordness::Term
    } else {
        Wordness::Name
    }
}

/// The text of the word `symbol` stands for, when it stands for one.
#[must_use]
pub fn term_text(symbol: Symbol) -> Option<NameText<'static>> {
    let within = symbol.0.checked_sub(TERM_BASE)?;
    // A place in a block past the words of a closed set is no word, which is the question asked.
    let Ok(number) = u8::try_from(within % BLOCK_SIZE) else {
        return None;
    };
    match within / BLOCK_SIZE {
        0 => DeclarationKind::from_repr(number).map(Term::text),
        1 => ParameterKind::from_repr(number).map(Term::text),
        2 => ValueKind::from_repr(number).map(Term::text),
        3 => MappingKind::from_repr(number).map(Term::text),
        4 => DependencyKind::from_repr(number).map(Term::text),
        6 => DependencyOrigin::from_repr(number).map(Term::text),
        _ => None,
    }
}
