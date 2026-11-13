//! The `names` service: free functions named by verb that make a name of pieces and place a name that leaves its file, with no IO and
//! no ports.

use super::types::NameStart;
use crate::domain::symbols::service::{bytes_of, hash_of, hash_of_symbol, insert_name};
use crate::domain::symbols::types::{ByteSlice, SymbolStorage, Symbols, SymbolsEngine};
use ryo_relations::domain::relations::types::{
    Bytes, Count, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Span, WorkerNumber,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;

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
    match symbols.storage.get(at) {
        Some(SymbolStorage::Source) => push_bytes(
            symbols,
            source,
            Offset(u32::try_from(start).unwrap_or(u32::MAX)),
            Offset(u32::try_from(end).unwrap_or(u32::MAX)),
        ),
        Some(SymbolStorage::Derived) if end <= symbols.derived.0.len() => {
            symbols.derived.0.extend_from_within(start..end);
        }
        Some(SymbolStorage::Derived) | None => {}
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
    let place = Span {
        start: Offset(u32::try_from(from.0).unwrap_or(u32::MAX)),
        end: Offset(u32::try_from(symbols.derived.0.len()).unwrap_or(u32::MAX)),
    };
    insert_name(symbols, place, hash, SymbolStorage::Derived)
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
    let Ok(number) = u32::try_from(places.len()) else {
        return None; // a file of more than 2^32 names has no number for the next one: it is not placed
    };
    let place = PlaceNumber(number);
    if symbols
        .storage
        .get(usize::try_from(symbol.0).unwrap_or(usize::MAX))
        == Some(&SymbolStorage::Source)
    {
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
