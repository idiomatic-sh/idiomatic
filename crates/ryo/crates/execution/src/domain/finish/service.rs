//! Normalize producer-local names at the owner-controlled publication boundary.
use super::types::{Arena, Leaves, Leaving, Retained};
use ryo_language::domain::symbols::{
    service::{intern_derived, name_made_across, name_of, text_of},
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{
    Bytes, NamePlace, Offset, Origin, Value, WorkerNumber,
};
use ryo_values::domain::files::types::SourceView;
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

/// Publish one independently decodable frame. Its place numbers never borrow the
/// caller's accumulated place log; only the backing text arena is shared.
pub(crate) fn escape_frame(
    table: &mut Table,
    leaving: Leaving<'_>,
    arena: Arena<'_>,
) -> Result<Vec<NamePlace>, super::types::PublicationReason> {
    use super::types::PublicationReason;
    use ryo_language::domain::symbols::service::{derived_of, hash_of, places_of};
    use ryo_relations::domain::relations::types::{CROSS_FILE, CROSS_PLACE, PlaceNumber};
    use ryo_relations::domain::terms::types::Symbol;

    if leaving.worker == CROSS_FILE {
        return Err(PublicationReason::ReservedArena);
    }
    let mut places = Vec::new();
    for value in &mut table.cells {
        let Value::Name(name) = *value else { continue };
        if name.place.0 & CROSS_PLACE != 0 {
            let symbol = Symbol(name.place.0 & !CROSS_PLACE);
            if name_made_across(leaving.symbols, symbol).map(|made| made.hash) != Some(name.hash) {
                return Err(PublicationReason::CrossNameMismatch);
            }
            *value = Value::Symbol(symbol);
            continue;
        }
        let mut place = *places_of(leaving.symbols)
            .get(name.place.0 as usize)
            .ok_or(PublicationReason::PlaceMissing)?;
        let range = place.start.0 as usize..place.end.0 as usize;
        let local = match place.origin {
            Origin::Source => {
                if place.file != leaving.file {
                    return Err(PublicationReason::SourceTextMissing);
                }
                Some(
                    leaving
                        .source
                        .0
                        .get(range)
                        .ok_or(PublicationReason::LocalRangeMissing)?,
                )
            }
            Origin::Derived(origin) if origin == CROSS_FILE => Some(
                derived_of(leaving.symbols)
                    .0
                    .get(range)
                    .ok_or(PublicationReason::LocalRangeMissing)?,
            ),
            Origin::Derived(_) => None,
        };
        if let Some(bytes) = local {
            if hash_of(ByteSlice(bytes)) != name.hash {
                return Err(PublicationReason::LocalHashMismatch);
            }
            let start = u32::try_from(arena.0.len())
                .map_err(|_| PublicationReason::StartUnrepresentable)?;
            let end = arena
                .0
                .len()
                .checked_add(bytes.len())
                .and_then(|end| u32::try_from(end).ok())
                .ok_or(PublicationReason::EndUnrepresentable)?;
            arena.0.extend_from_slice(bytes);
            place.origin = Origin::Derived(leaving.worker);
            place.start = Offset(start);
            place.end = Offset(end);
        }
        let index =
            u32::try_from(places.len()).map_err(|_| PublicationReason::PortablePlaceLogFull)?;
        if index >= CROSS_PLACE {
            return Err(PublicationReason::PortablePlaceLogFull);
        }
        places.push(place);
        *value = Value::Name(ryo_relations::domain::relations::types::Name {
            hash: name.hash,
            place: PlaceNumber(index),
        });
    }
    escape(
        table,
        leaving,
        Retained {
            arena,
            places: &mut places,
        },
    );
    if table
        .cells
        .iter()
        .any(|value| matches!(value, Value::Symbol(_)))
    {
        return Err(PublicationReason::ProducerSymbolUnescaped);
    }
    Ok(places)
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
