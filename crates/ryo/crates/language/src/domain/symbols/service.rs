// ignore: idiomatic.arch/file-length - every public function here is a path the SDK hands languages (named in the product's tastemaker.toml), over the one private `SymbolsEngine` they reach through `AsRef`/`AsMut`; the engine-level helpers beside them (interning, hashing, deriving) are what those functions share, and moving either half would move SDK paths or split a function from the private table it reads.
//! The `symbols` service: free functions named by verb over a file's names, with no IO and no
//! ports. A name is a number: its text is in the file or, when it is made, beside it.

use crate::domain::terms::types::Wordness;
use std::hash::Hasher as _;

use hashbrown::hash_table::OccupiedEntry;
use rustc_hash::FxHasher;

use crate::domain::relations::constants::NO_SYMBOL;
use crate::domain::relations::types::{
    Bits, Bytes, CROSS_FILE, CROSS_PLACE, Count, FileId, KindBits, Name, NameNote, NamePlace,
    Offset, Origin, PlaceNumber, Span, TextHash, WorkerNumber,
};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::SymbolsEngine;
use crate::domain::symbols::types::{NameStart, Noted, QuerySymbols, Symbols};
use crate::domain::terms::service::{is_term, term_text};
use crate::domain::terms::types::NameText;
use crate::domain::terms::types::Symbol;

/// Empties the names, keeping their room for the next file.
pub fn clear_symbols(symbols: &mut Symbols) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    symbols.start.clear();
    symbols.end.clear();
    symbols.hash.clear();
    symbols.index.clear();
    symbols.sources = Count(0);
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
    let holder = if usize::try_from(symbol.0).unwrap_or(usize::MAX) < symbols.sources.0 {
        source
    } else {
        ByteSlice(&symbols.derived.0)
    };
    holder.0.get(start..end).map(ByteSlice)
}

/// The name of a symbol made by the steps that run after the files were read, as a name that has left its file:
/// its text stays in the derived text of `symbols`, and its place is that symbol.
#[must_use]
pub fn name_made_across(symbols: &Symbols, symbol: Symbol) -> Option<Name> {
    let symbols: &SymbolsEngine = symbols.as_ref();
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
    std::str::from_utf8(bytes_of(symbols.as_ref(), source, symbol)?.0)
        .ok() // ignore: idiomatic.arch/no-ok-on-result - a name's bytes that are not text have no text, which the caller reads as no name text: the rows keep the name's place either way
        .map(NameText)
}

/// The hash of the text of the name `symbol` stands for.
#[must_use]
pub fn hash_in(symbols: &Symbols, symbol: Symbol) -> Option<TextHash> {
    hash_of_symbol(symbols.as_ref(), symbol)
}

/// The hash of the text of the name `symbol` stands for, read from the names as the engine keeps them.
fn hash_of_symbol(symbols: &SymbolsEngine, symbol: Symbol) -> Option<TextHash> {
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
    add: impl FnOnce(&mut SymbolsEngine) -> Span,
) -> Symbol {
    find_or_add_hashed(symbols, source, text, hash_of(text), add)
}

/// Finds an already-interned file or derived symbol by hash and exact bytes, without adding a name.
/// Closed-set terms are not in this index; callers comparing by text must check their aliases too.
#[must_use]
pub fn find_hashed(
    symbols: &Symbols,
    source: ByteSlice<'_>,
    text: ByteSlice<'_>,
    hash: TextHash,
) -> Option<Symbol> {
    find_hashed_in(symbols.as_ref(), source, text, hash)
}

/// The shared read-only lookup behind `find_hashed` and interning.
fn find_hashed_in(
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
    add: impl FnOnce(&mut SymbolsEngine) -> Span,
) -> Symbol {
    let known = &*symbols;
    if let Some(found) = find_hashed_in(known, source, text, hash) {
        return found;
    }
    let symbol = Symbol(u32::try_from(symbols.start.len()).unwrap_or(u32::MAX));
    let place = add(symbols);
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
    let symbol = find_or_add(symbols, source, text, |_| Span { start, end });
    symbols.sources = Count(symbols.start.len());
    symbol
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
/// use ryo_language::domain::terms::types::NameText;
/// let mut file = Symbols::default();
/// let mut query = QuerySymbols::from(&mut file);
/// let made = query_intern_derived(&mut query, ByteSlice(b""), NameText("name"));
/// assert_eq!(text_of(query.as_ref(), ByteSlice(b""), made).map(|text| text.0), Some("name"));
/// ```
///
/// ```compile_fail
/// use ryo_language::domain::symbols::service::{intern, query_intern_derived, text_of};
/// use ryo_language::domain::symbols::types::{ByteSlice, QuerySymbols, Symbols};
/// use ryo_language::domain::relations::types::Offset;
/// use ryo_language::domain::terms::types::NameText;
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
    find_or_add(symbols, source, ByteSlice(text.as_bytes()), |symbols| {
        let start = Offset(u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX));
        symbols.derived.0.extend_from_slice(text.as_bytes());
        Span {
            start,
            end: Offset(u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX)),
        }
    })
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

/// Begins a name made of pieces: what is pushed next, up to [`end_name`], is its text.
#[must_use]
pub fn begin_name(symbols: &Symbols) -> NameStart {
    let symbols: &SymbolsEngine = symbols.as_ref();
    NameStart(symbols.derived.0.len())
}

/// Adds `text` to the name being made.
pub fn push_text(symbols: &mut Symbols, text: NameText<'_>) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    symbols.derived.0.extend_from_slice(text.0.as_bytes());
}

/// Adds the bytes `start..end` of the file to the name being made.
pub fn push_span(symbols: &mut Symbols, source: ByteSlice<'_>, start: Offset, end: Offset) {
    push_bytes(symbols.as_mut(), source, start, end);
}

/// Adds the bytes `start..end` of the file to the name being made, in the names as the engine keeps them.
fn push_bytes(symbols: &mut SymbolsEngine, source: ByteSlice<'_>, start: Offset, end: Offset) {
    let text = source
        .0
        .get(
            usize::try_from(start.0).unwrap_or(usize::MAX)
                ..usize::try_from(end.0).unwrap_or(usize::MAX),
        )
        .unwrap_or(&[]);
    symbols.derived.0.extend_from_slice(text);
}

/// Adds the text of the name `symbol` to the name being made.
pub fn push_name(symbols: &mut Symbols, source: ByteSlice<'_>, symbol: Symbol) {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    let at = usize::try_from(symbol.0).unwrap_or(usize::MAX);
    let (Some(start), Some(end)) = (symbols.start.get(at).copied(), symbols.end.get(at).copied())
    else {
        return;
    };
    let (start, end) = (
        usize::try_from(start.0).unwrap_or(usize::MAX),
        usize::try_from(end.0).unwrap_or(usize::MAX),
    );
    if usize::try_from(symbol.0).unwrap_or(usize::MAX) < symbols.sources.0 {
        push_bytes(
            symbols,
            source,
            Offset(u32::try_from(start).unwrap_or(u32::MAX)),
            Offset(u32::try_from(end).unwrap_or(u32::MAX)),
        );
        return;
    }
    // A derived name's text is in the derived text, unless what was pushed of it was taken back.
    if end <= symbols.derived.0.len() {
        symbols.derived.0.extend_from_within(start..end);
    }
}

/// Ends the name begun at `from`: its text is what was pushed. The same text is always the same
/// symbol, so a name the file already has is found and what was pushed is taken back.
pub fn end_name(symbols: &mut Symbols, source: ByteSlice<'_>, from: NameStart) -> Symbol {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    let text = ByteSlice(symbols.derived.0.get(from.0..).unwrap_or(&[]));
    let hash = hash_of(text);
    let known = &*symbols;
    if let Some(found) = known
        .index
        .find(hash.0, |symbol| {
            bytes_of(known, source, *symbol) == Some(text)
        })
        .copied()
    {
        symbols.derived.0.truncate(from.0);
        return found;
    }
    let symbol = Symbol(u32::try_from(symbols.start.len()).unwrap_or(u32::MAX));
    symbols
        .start
        .push(Offset(u32::try_from(from.0).unwrap_or(u32::MAX)));
    symbols.end.push(Offset(
        u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX),
    ));
    symbols.hash.push(hash);
    let hashes = &symbols.hash;
    let _: OccupiedEntry<'_, Symbol> = symbols.index.insert_unique(hash.0, symbol, |symbol| {
        symbol_in(hashes, *symbol).copied().unwrap_or_default().0
    });
    symbol
}

/// The name a symbol of `file` becomes when it leaves the file, its place pushed onto `places`. A derived name's text is copied
/// once into `arena`, the worker's own text, which the worker keeps for the whole run.
#[must_use]
pub fn name_of(
    symbols: &Symbols,
    source: ByteSlice<'_>,
    symbol: Symbol,
    file: FileId,
    worker: WorkerNumber,
    arena: &mut Bytes,
    places: &mut Vec<NamePlace>,
) -> Option<Name> {
    let symbols: &SymbolsEngine = symbols.as_ref();
    let bytes = bytes_of(symbols, source, symbol)?;
    let hash = hash_of_symbol(symbols, symbol)?;
    let place = PlaceNumber(u32::try_from(places.len()).ok()?); // ignore: idiomatic.arch/no-ok-on-result - a file of more than 2^32 names has no number for the next one: it is not placed
    if usize::try_from(symbol.0).unwrap_or(usize::MAX) < symbols.sources.0 {
        let (start, end) = (
            *symbols
                .start
                .get(usize::try_from(symbol.0).unwrap_or(usize::MAX))?,
            *symbols
                .end
                .get(usize::try_from(symbol.0).unwrap_or(usize::MAX))?,
        );
        places.push(NamePlace {
            origin: Origin::Source,
            file,
            start,
            end,
        });
        return Some(Name { hash, place });
    }
    let start = u32::try_from(arena.0.len()).unwrap_or(u32::MAX);
    arena.0.extend_from_slice(bytes.0);
    places.push(NamePlace {
        origin: Origin::Derived(worker),
        file,
        start: Offset(start),
        end: Offset(u32::try_from(arena.0.len()).unwrap_or(u32::MAX)),
    });
    Some(Name { hash, place })
}

/// Where the text of each name that left its file is, by the number of the place.
#[must_use]
pub fn places_of(symbols: &Symbols) -> &[NamePlace] {
    let symbols: &SymbolsEngine = symbols.as_ref();
    &symbols.places
}

/// The text the names made after their file was read are in.
#[must_use]
pub fn derived_of(symbols: &Symbols) -> &Bytes {
    let symbols: &SymbolsEngine = symbols.as_ref();
    &symbols.derived
}

/// How many names the file has so far, the ones in its bytes and the ones made.
#[must_use]
pub fn name_count(symbols: &Symbols) -> Count {
    let symbols: &SymbolsEngine = symbols.as_ref();
    Count(symbols.start.len())
}

/// The number a reader noted about the name `name` while reading the file; nothing noted is `0`.
#[must_use]
pub fn note_of(symbols: &Symbols, name: Symbol) -> NameNote {
    let symbols: &SymbolsEngine = symbols.as_ref();
    symbols // ignore: idiomatic.arch/no-unwrap-or-on-lookup - the notes hold only the names a reader noted: a name noted nothing of is `0`, as the doc says
        .notes
        .indexed
        .get(usize::try_from(name.0).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default()
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

/// Where the text of each name that left its file is, to write: the steps after the files were read make the places of their own.
pub fn places_mut(symbols: &mut Symbols) -> &mut Vec<NamePlace> {
    let symbols: &mut SymbolsEngine = symbols.as_mut();
    &mut symbols.places
}
