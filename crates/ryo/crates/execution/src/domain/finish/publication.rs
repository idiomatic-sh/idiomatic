//! Make a cross-stage table portable before its producer-local symbol table is released.
use super::{
    service::escape,
    types::{Arena, Leaves, Leaving, PublicationReason, Retained},
};
use ryo_language::domain::symbols::{service::{hash_in, hash_of, intern_derived}, types::{ByteSlice, Symbols}};
use ryo_language::domain::names::service::{derived_of, places_of};
use ryo_relations::domain::{
    relations::types::{
        CROSS_FILE, CROSS_PLACE, FileId, NamePlace, Origin, PlaceNumber, Value, WorkerNumber,
    },
    terms::types::{NameText, Symbol},
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::collections::BTreeMap;

/// Normalize local names, preserve real inherited origins, then use the ordinary escape boundary.
/// The caller owns the table boundary, mutable producer Symbols and destination arena; Emit is not aliased.
pub(crate) fn escape_cross_publication(
    table: &mut Table,
    producer: &mut Symbols,
    inherited: &[ArenaBytes],
    worker: WorkerNumber,
    arena: &mut ArenaBytes,
    places: &mut Vec<NamePlace>,
) -> Result<(), PublicationReason> {
    if worker == CROSS_FILE || !places.is_empty() {
        return Err(refused(PublicationReason::ReservedArena));
    }
    let previous = places_of(producer).to_vec();
    let mut remapped = BTreeMap::new();
    for value in &mut table.cells {
        let Value::Name(name) = *value else {
            continue;
        };
        // Name is packed. Copy fields before comparisons or borrowing map keys.
        let name_hash = name.hash;
        let name_place = name.place;
        if name_place.0 & CROSS_PLACE != 0 {
            let symbol = Symbol(name_place.0 & !CROSS_PLACE);
            if hash_in(producer, symbol) != Some(name_hash) {
                return Err(refused(PublicationReason::CrossNameMismatch));
            }
            *value = Value::Symbol(symbol);
            continue;
        }
        let old = previous
            .get(
                usize::try_from(name_place.0)
                    .map_err(|_| refused(PublicationReason::PlaceUnrepresentable))?,
            )
            .ok_or_else(|| refused(PublicationReason::PlaceMissing))?;
        let Origin::Derived(origin) = old.origin else {
            return Err(refused(PublicationReason::SourceTextMissing));
        };
        let range = usize::try_from(old.start.0)
            .map_err(|_| refused(PublicationReason::StartUnrepresentable))?
            ..usize::try_from(old.end.0)
                .map_err(|_| refused(PublicationReason::EndUnrepresentable))?;
        if origin == CROSS_FILE {
            // End the immutable derived-text borrow before interning mutably into the same Symbols.
            let owned = derived_of(producer)
                .0
                .get(range)
                .ok_or_else(|| refused(PublicationReason::LocalRangeMissing))?
                .to_vec();
            if hash_of(ByteSlice(&owned)) != name_hash {
                return Err(refused(PublicationReason::LocalHashMismatch));
            }
            let text = std::str::from_utf8(&owned)
                .map_err(|_| refused(PublicationReason::LocalTextInvalid))?;
            let symbol = intern_derived(producer, ByteSlice(&[]), NameText(text));
            if hash_in(producer, symbol) != Some(name_hash) {
                return Err(refused(PublicationReason::NormalizedIdentityMismatch));
            }
            *value = Value::Symbol(symbol);
        } else {
            let bytes = inherited
                .get(
                    usize::try_from(origin.0)
                        .map_err(|_| refused(PublicationReason::InheritedArenaUnrepresentable))?,
                )
                .and_then(|arena| arena.0.get(range))
                .ok_or_else(|| refused(PublicationReason::InheritedRangeMissing))?;
            if hash_of(ByteSlice(bytes)) != name_hash || std::str::from_utf8(bytes).is_err() {
                return Err(refused(PublicationReason::InheritedTextMismatch));
            }
            let at = if let Some(at) = remapped.get(&name_place) {
                *at
            } else {
                let at = u32::try_from(places.len())
                    .map_err(|_| refused(PublicationReason::PortablePlaceLogFull))?;
                if at >= CROSS_PLACE {
                    return Err(refused(PublicationReason::PortablePlaceLogFull));
                }
                let at = PlaceNumber(at);
                places.push(*old);
                remapped.insert(name_place, at);
                at
            };
            *value = Value::Name(ryo_relations::domain::relations::types::Name {
                hash: name_hash,
                place: at,
            });
        }
    }
    escape(
        table,
        Leaving {
            symbols: producer,
            source: SourceView(&[]),
            file: FileId(u32::MAX),
            worker,
            text: Leaves::WithText,
        },
        Retained {
            arena: Arena(&mut arena.0),
            places,
        },
    );
    for value in &table.cells {
        match value {
            Value::Symbol(_) => return Err(refused(PublicationReason::ProducerSymbolUnescaped)),
            Value::Name(name) => {
                let name_place = name.place;
                if name_place.0 & CROSS_PLACE != 0 {
                    return Err(refused(PublicationReason::CrossPlaceUnescaped));
                }
                let place = places
                    .get(
                        usize::try_from(name_place.0)
                            .map_err(|_| refused(PublicationReason::EscapedPlaceUnrepresentable))?,
                    )
                    .ok_or_else(|| refused(PublicationReason::EscapedPlaceMissing))?;
                if !matches!(place.origin, Origin::Derived(origin) if origin != CROSS_FILE) {
                    return Err(refused(PublicationReason::EscapedArenaUnstable));
                }
            }
            Value::Node(_)
            | Value::File(_)
            | Value::Span(_)
            | Value::Number(_)
            | Value::Flag(_) => {}
        }
    }
    Ok(())
}
fn refused(reason: PublicationReason) -> PublicationReason {
    reason
}
