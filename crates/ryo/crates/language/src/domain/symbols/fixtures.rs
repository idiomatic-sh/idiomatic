//! The test support of the `symbols` concept: the symbol calls the tests of other crates make.

use super::service;
use super::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{Name, NamePlace, Offset, TextHash};
use ryo_relations::domain::terms::types::{NameText, Symbol};

/// The hash of a text.
#[must_use]
pub fn hash_of(text: ByteSlice<'_>) -> TextHash {
    service::hash_of(text)
}

/// The hash of the text a symbol stands for, when it has one.
#[must_use]
pub fn hash_in(symbols: &Symbols, symbol: Symbol) -> Option<TextHash> {
    service::hash_in(symbols, symbol)
}

/// The symbol of the source between two offsets.
pub fn intern(symbols: &mut Symbols, source: ByteSlice<'_>, start: Offset, end: Offset) -> Symbol {
    service::intern(symbols, source, start, end)
}

/// The symbol of a derived name.
pub fn intern_derived(symbols: &mut Symbols, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {
    service::intern_derived(symbols, source, text)
}

/// The name a symbol was made across files, when it was.
#[must_use]
pub fn name_made_across(symbols: &Symbols, symbol: Symbol) -> Option<Name> {
    service::name_made_across(symbols, symbol)
}

/// The places of the names of a table, for a test to read or damage.
pub fn places_mut(symbols: &mut Symbols) -> &mut Vec<NamePlace> {
    service::places_mut(symbols)
}
