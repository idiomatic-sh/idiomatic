//! The `symbols` service: free functions named by verb over a file's names, with no IO and no
//! ports. A name is a number: its text is in the file or, when it is made, beside it.

use ryo_relations::domain::terms::types::Wordness;
use std::hash::Hasher as _;

use hashbrown::hash_table::OccupiedEntry;
use rustc_hash::FxHasher;

use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::{Generation, QuerySymbols, Symbols};
use crate::domain::symbols::types::{SymbolStorage, SymbolsEngine};
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::types::{
    CROSS_FILE, CROSS_PLACE, FileId, KindBits, Name, NamePlace, Offset, Origin, PlaceNumber, Span,
    TextHash,
};
use ryo_relations::domain::terms::service::{is_term, term_text};
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;

/// Empties the names, keeping their room for the next file.
pub fn clear_symbols(symbols: &mut Symbols) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    symbols.transport_generation = std::sync::Arc::new(());
    symbols.start.clear();
    symbols.end.clear();
    symbols.hash.clear();
    symbols.index.clear();
    symbols.storage.clear();
    symbols.derived.0.clear();
    symbols.vocabulary.clear();
    symbols.places.clear();
    symbols.notes.indexed.clear();
    symbols.notes.listed.0.clear();
}

/// The one of `items` that the name `symbol` stands for, when there is one.
#[must_use]
pub(crate) fn symbol_in<T>(items: &[T], symbol: Symbol) -> Option<&T> {
    items.get(usize::try_from(symbol.0).unwrap_or(usize::MAX))
}

/// The one of `items` that the file `file` stands for, when there is one.
#[must_use]
pub fn file_in<T>(items: &[T], file: FileId) -> Option<&T> {
    items.get(usize::try_from(file.0).unwrap_or(usize::MAX))
}

/// The hash of a text, which finds its symbol among the names.
#[must_use]
pub fn hash_of(text: ByteSlice<'_>) -> TextHash {
    let mut hasher = FxHasher::default();
    hasher.write(text.0);
    TextHash(hasher.finish())
}

/// The bytes of the name `symbol` stands for: in the file's bytes, or in the derived text.
#[must_use]
pub(crate) fn bytes_of<'a>(
    symbols: &'a SymbolsEngine,
    source: ByteSlice<'a>,
    symbol: Symbol,
) -> Option<ByteSlice<'a>> {
    if is_term(symbol) == Wordness::Term {
        return term_text(symbol).map(|text| ByteSlice(text.0.as_bytes()));
    }
    let at = usize::try_from(symbol.0).unwrap_or(usize::MAX);
    let (start, end) = (
        usize::try_from(symbols.start.get(at)?.0).unwrap_or(usize::MAX),
        usize::try_from(symbols.end.get(at)?.0).unwrap_or(usize::MAX),
    );
    let holder = match symbols.storage.get(at)? {
        SymbolStorage::Source => source,
        SymbolStorage::Derived => ByteSlice(&symbols.derived.0),
    };
    holder.0.get(start..end).map(ByteSlice)
}

/// The name of a symbol made by the steps that run after the files were read, as a name that has left its file:
/// its text stays in the derived text of `symbols`, and its place is that symbol.
/// A source-owned or unknown symbol has no cross-file derived place and is refused.
#[must_use]
pub fn name_made_across(symbols: &Symbols, symbol: Symbol) -> Option<Name> {
    let symbols: &SymbolsEngine = symbols.as_ref();
    let Ok(at) = usize::try_from(symbol.0) else {
        return None;
    };
    if symbols.storage.get(at) != Some(&SymbolStorage::Derived) {
        return None;
    }
    Some(Name {
        hash: hash_of_symbol(symbols, symbol)?,
        place: PlaceNumber(CROSS_PLACE | symbol.0),
    })
}

/// Where the text of `name` is: among `places`, or among the symbols the steps after the files made when the name is one of theirs.
#[must_use]
pub fn place_of(name: Name, places: &[NamePlace], across: &Symbols) -> Option<NamePlace> {
    let number = name.place;
    if number.0 & CROSS_PLACE != 0 {
        let at = usize::try_from(number.0 & !CROSS_PLACE).unwrap_or(usize::MAX);
        if AsRef::<SymbolsEngine>::as_ref(across).storage.get(at) != Some(&SymbolStorage::Derived) {
            return None;
        }
        return Some(NamePlace {
            origin: Origin::Derived(CROSS_FILE),
            file: FileId(u32::MAX),
            start: *AsRef::<SymbolsEngine>::as_ref(across).start.get(at)?,
            end: *AsRef::<SymbolsEngine>::as_ref(across).end.get(at)?,
        });
    }
    places
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
        .copied()
}

/// The text of the name `symbol` stands for.
#[must_use]
pub fn text_of<'a>(
    symbols: &'a Symbols,
    source: ByteSlice<'a>,
    symbol: Symbol,
) -> Option<NameText<'a>> {
    // A name's bytes that are not text have no text, which the caller reads as no name text: the rows keep the name's place either way.
    match std::str::from_utf8(bytes_of(symbols.as_ref(), source, symbol)?.0) {
        Ok(text) => Some(NameText(text)),
        Err(_) => None,
    }
}

/// The hash of the text of the name `symbol` stands for.
#[must_use]
pub fn hash_in(symbols: &Symbols, symbol: Symbol) -> Option<TextHash> {
    hash_of_symbol(symbols.as_ref(), symbol)
}

/// The hash of the text of the name `symbol` stands for, read from the names as the engine keeps them.
pub(crate) fn hash_of_symbol(symbols: &SymbolsEngine, symbol: Symbol) -> Option<TextHash> {
    if is_term(symbol) == Wordness::Term {
        return term_text(symbol).map(|text| hash_of(ByteSlice(text.0.as_bytes())));
    }
    symbol_in(&symbols.hash, symbol).copied()
}

/// Finds the symbol of the name whose bytes are `text`, or adds `add` as a new one.
fn find_or_add(
    symbols: &mut SymbolsEngine,
    source: ByteSlice<'_>,
    text: ByteSlice<'_>,
    storage: SymbolStorage,
    add: impl FnOnce(&mut SymbolsEngine) -> Span,
) -> Symbol {
    find_or_add_hashed(symbols, source, text, hash_of(text), storage, add)
}

/// Finds an already-interned file or derived symbol by hash and exact bytes, without adding a name: the read-only lookup
/// behind interning. Closed-set terms are not in this index; callers comparing by text must check their aliases too.
#[must_use]
pub(crate) fn find_hashed_in(
    symbols: &SymbolsEngine,
    source: ByteSlice<'_>,
    text: ByteSlice<'_>,
    hash: TextHash,
) -> Option<Symbol> {
    symbols
        .index
        .find(hash.0, |symbol| {
            bytes_of(symbols, source, *symbol) == Some(text)
        })
        .copied()
}

/// Finds the symbol of the name whose bytes are `text`, which hashes to `hash`, or adds it: a hit is the name only when its text is
/// `text` too, so two texts of one hash stay two names.
pub(crate) fn find_or_add_hashed(
    symbols: &mut SymbolsEngine,
    source: ByteSlice<'_>,
    text: ByteSlice<'_>,
    hash: TextHash,
    storage: SymbolStorage,
    add: impl FnOnce(&mut SymbolsEngine) -> Span,
) -> Symbol {
    let known = &*symbols;
    if let Some(found) = find_hashed_in(known, source, text, hash) {
        return found;
    }
    let place = add(symbols);
    insert_name(symbols, place, hash, storage)
}

/// Append the complete identity together: no insertion path may omit its byte owner.
pub(crate) fn insert_name(
    symbols: &mut SymbolsEngine,
    place: Span,
    hash: TextHash,
    storage: SymbolStorage,
) -> Symbol {
    let symbol = Symbol(u32::try_from(symbols.start.len()).unwrap_or(u32::MAX));
    symbols.storage.push(storage);
    symbols.start.push(place.start);
    symbols.end.push(place.end);
    symbols.hash.push(hash);
    let hashes = &symbols.hash;
    let _: OccupiedEntry<'_, Symbol> = symbols.index.insert_unique(hash.0, symbol, |symbol| {
        symbol_in(hashes, *symbol).copied().unwrap_or_default().0
    });
    symbol
}

/// The symbol for the name at `start..end` of the file's bytes `source`: the same text is always
/// the same symbol within one file. A new name is a place and a hash; nothing is copied.
pub fn intern(symbols: &mut Symbols, source: ByteSlice<'_>, start: Offset, end: Offset) -> Symbol {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    let text = ByteSlice(
        source
            .0
            .get(
                usize::try_from(start.0).unwrap_or(usize::MAX)
                    ..usize::try_from(end.0).unwrap_or(usize::MAX),
            )
            .unwrap_or(&[]),
    );
    find_or_add(symbols, source, text, SymbolStorage::Source, |_| Span {
        start,
        end,
    })
}

/// The symbol for a name that is not in the file's bytes: its text is kept in the derived text.
pub fn intern_derived(symbols: &mut Symbols, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {
    derive(symbols.as_mut(), source, text)
}

/// Reborrows a query's symbols without handing a query operator source-name interning.
#[must_use]
pub fn query_reborrow<'a>(symbols: &'a mut QuerySymbols<'_>) -> QuerySymbols<'a> {
    QuerySymbols(&mut *symbols.0)
}

/// Adds a value computed by a query, retaining existing source and derived identities.
///
/// ```
/// use ryo_language::domain::symbols::service::{query_intern_derived, text_of};
/// use ryo_language::domain::symbols::types::{ByteSlice, QuerySymbols, Symbols};
/// use ryo_relations::domain::terms::types::NameText;
/// let mut file = Symbols::default();
/// let mut query = QuerySymbols::from(&mut file);
/// let made = query_intern_derived(&mut query, ByteSlice(b""), NameText("name"));
/// assert_eq!(text_of(query.as_ref(), ByteSlice(b""), made).map(|text| text.0), Some("name"));
/// ```
///
/// ```compile_fail,E0308
/// use ryo_language::domain::symbols::service::{intern, query_intern_derived, text_of};
/// use ryo_language::domain::symbols::types::{ByteSlice, QuerySymbols, Symbols};
/// use ryo_relations::domain::relations::types::Offset;
/// use ryo_relations::domain::terms::types::NameText;
/// let mut file = Symbols::default();
/// let mut query = QuerySymbols::from(&mut file);
/// let made = query_intern_derived(&mut query, ByteSlice(b""), NameText("name"));
/// let _ = text_of(query.as_ref(), ByteSlice(b""), made);
/// intern(&mut query, ByteSlice(b"name"), Offset(0), Offset(4));
/// ```
pub fn query_intern_derived(
    symbols: &mut QuerySymbols<'_>,
    source: ByteSlice<'_>,
    text: NameText<'_>,
) -> Symbol {
    intern_derived(&mut *symbols.0, source, text)
}

/// The symbol for a name that is not in the file's bytes, in the names as the engine keeps them: its text is kept in the derived text.
fn derive(symbols: &mut SymbolsEngine, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {
    let text = text.0;
    find_or_add(
        symbols,
        source,
        ByteSlice(text.as_bytes()),
        SymbolStorage::Derived,
        |symbols| {
            let start = Offset(u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX));
            symbols.derived.0.extend_from_slice(text.as_bytes());
            Span {
                start,
                end: Offset(u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX)),
            }
        },
    )
}

/// The name numbered `number` in a language's vocabulary, made from `text` the first time a file
/// asks for it and found by its number after: a kind's name is read once for a file, not for each
/// element of that kind.
pub fn vocabulary_symbol(
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    number: KindBits,
    text: NameText<'_>,
) -> Symbol {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    let at = usize::from(number.0);
    if let Some(known) = symbols
        .vocabulary
        .get(at)
        .copied()
        .filter(|each| *each != NO_SYMBOL)
    {
        return known;
    }
    let made = derive(symbols, source, text);
    if symbols.vocabulary.len() <= at {
        symbols.vocabulary.resize(at + 1, NO_SYMBOL);
    }
    if let Some(slot) = symbols.vocabulary.get_mut(at) {
        *slot = made;
    }
    made
}

/// Where the text of each name that left its file is, to write: the steps after the files were read make the places of their own.
pub fn places_mut(symbols: &mut Symbols) -> &mut Vec<NamePlace> {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    &mut symbols.places
}

/// Mint a frame-admission key before the query borrows this exact symbol pool.
/// The key is held by the application input owner and is not given to kernels.
pub fn transport_permit(symbols: &mut Symbols) -> super::types::TransportPermit {
    let symbols: &SymbolsEngine = symbols.as_ref();
    super::types::TransportPermit(std::sync::Arc::clone(&symbols.transport_generation))
}

/// Whether two engine-issued frame permits name the same live symbol generation.
/// Comparing permits exposes no place mutation to a query or semantic kernel.
pub fn same_transport_generation(
    first: &super::types::TransportPermit,
    second: &super::types::TransportPermit,
) -> Generation {
    if std::sync::Arc::ptr_eq(&first.0, &second.0) {
        Generation::Same
    } else {
        Generation::Different
    }
}

/// Check a semantic query borrow against an application-held frame key without
/// granting the query a way to mint keys or mutate transported name places.
pub fn query_has_transport_generation(
    symbols: &QuerySymbols<'_>,
    permit: &super::types::TransportPermit,
) -> Generation {
    let pool: &SymbolsEngine = symbols.0.as_ref();
    if std::sync::Arc::ptr_eq(&pool.transport_generation, &permit.0) {
        Generation::Same
    } else {
        Generation::Different
    }
}

/// Let the owner decode one demanded transport frame in place. QuerySymbols
/// alone cannot call this: it cannot mint the key, and a stale/foreign key
/// answers nothing before any place is appended.
pub fn query_places_mut<'a>(
    symbols: &'a mut QuerySymbols<'_>,
    permit: &super::types::TransportPermit,
) -> Option<&'a mut Vec<NamePlace>> {
    let pool: &mut SymbolsEngine = symbols.0.as_mut();
    if !std::sync::Arc::ptr_eq(&pool.transport_generation, &permit.0) {
        return None;
    }
    Some(&mut pool.places)
}
