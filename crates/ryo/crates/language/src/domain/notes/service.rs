//! The `notes` service: free functions named by verb over what readers noted about the names of a file, with no IO and no ports.

use super::types::Noted;
use crate::domain::symbols::types::{Symbols, SymbolsEngine};
use ryo_relations::domain::relations::types::{Bits, NameNote};
use ryo_relations::domain::terms::types::Symbol;

/// The number a reader noted about the name `name` while reading the file; nothing noted is `0`.
#[must_use]
pub fn note_of(symbols: &Symbols, name: Symbol) -> NameNote {
    let symbols: &SymbolsEngine = symbols.as_ref();
    // The notes hold only the names a reader noted: a name noted nothing of is `0`, as the doc says.
    match symbols
        .notes
        .indexed
        .get(usize::try_from(name.0).unwrap_or(usize::MAX))
    {
        Some(note) => *note,
        None => NameNote::default(),
    }
}

/// Notes the number `value` about the name `name`, for the rest of the file.
pub fn set_note(symbols: &mut Symbols, name: Symbol, value: NameNote) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    let place = usize::try_from(name.0).unwrap_or(usize::MAX);
    if symbols.notes.indexed.len() <= place {
        symbols.notes.indexed.resize(place + 1, NameNote(0));
    }
    if let Some(slot) = symbols.notes.indexed.get_mut(place) {
        *slot = value;
    }
}

/// Whether a reader has noted any word while reading the file.
#[must_use]
pub fn has_noted_words(symbols: &Symbols) -> Noted {
    let symbols: &SymbolsEngine = symbols.as_ref();
    if symbols.notes.listed.0.is_empty() {
        Noted::Nothing
    } else {
        Noted::Words
    }
}

/// Notes a word for the rest of the file.
pub fn note_word(symbols: &mut Symbols, word: Bits) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    symbols.notes.listed.0.push(word.0);
}
