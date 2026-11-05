//! Normalize producer-local names at the owner-controlled publication boundary.
use super::types::{Arena, Leaves, Leaving, Retained};
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::{
    service::{intern_derived, name_made_across, name_of, text_of},
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{
    Bytes, NamePlace, Offset, Origin, Value, WorkerNumber,
};
use ryo_values::domain::rows::types::Table;
/// Turns each name of the rows into a name that leaves its file, in place: its text stays where
/// it is, and the row carries the hash of the text and the number of its place, which is pushed onto `places`.
pub(crate) fn escape(table: &mut Table, leaving: Leaving<'_>, kept: Retained<'_>) {
    let Leaving {
        symbols,
        source,
        file,
        worker,
        text,
    } = leaving;
    let Retained { arena, places } = kept;
    for value in &mut table.cells {
        if let Value::Symbol(symbol) = value {
            let mut made = Bytes(std::mem::take(&mut *arena.0));
            let named = name_of(
                symbols,
                ByteSlice(source.0),
                *symbol,
                file,
                worker,
                &mut made,
                places,
            );
            *arena.0 = made.0;
            let Some(name) = named else { continue };
            if text == Leaves::WithText
                && let Some(place) = places.last_mut()
            {
                *place = in_arena(*place, source, worker, Arena(&mut *arena.0));
            }
            *value = Value::Name(name);
        }
    }
}

/// Turns each name of a table the project wrote (its text is in `from`) into a name of the steps that run after
/// the files were read, whose text is in `cross`: it compares with the names the files handed over.
pub(crate) fn carry(table: &mut Table, from: &Symbols, cross: &mut Symbols) {
    for value in &mut table.cells {
        if let Value::Symbol(symbol) = value
            && let Some(text) = text_of(from, ByteSlice(&[]), *symbol)
        {
            let made = intern_derived(cross, ByteSlice(&[]), text);
            if let Some(name) = name_made_across(cross, made) {
                *value = Value::Name(name);
            }
        }
    }
}

/// A place whose text is in the file, made one whose text is in the worker's arena: steps that read the text
/// after the files were read cannot reach the file.
fn in_arena(
    place: NamePlace,
    source: SourceView<'_>,
    worker: WorkerNumber,
    arena: Arena<'_>,
) -> NamePlace {
    let arena = arena.0;
    if place.origin != Origin::Source {
        return place;
    }
    let text = source
        .0
        .get(
            usize::try_from(place.start.0).unwrap_or(usize::MAX)
                ..usize::try_from(place.end.0).unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    let start = u32::try_from(arena.len()).unwrap_or(u32::MAX);
    arena.extend_from_slice(text);
    NamePlace {
        origin: Origin::Derived(worker),
        start: Offset(start),
        end: Offset(u32::try_from(arena.len()).unwrap_or(u32::MAX)),
        ..place
    }
}
