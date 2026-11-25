//! The `spelling` service: whether the names the files handed over are spelled alike, with no IO and no ports.

use super::errors::SpellingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::ArenaBytes;

use ryo_language::domain::names::service::{derived_of, places_of};
use ryo_language::domain::symbols::service::place_of;
use ryo_relations::domain::relations::types::{CROSS_FILE, Origin, TextHash};


/// The same spelling invariant over compact admission's names; no row table is required.
///
/// # Errors
///
/// [`SpellingError::EntityInTwoSources`] when two names of one hash are spelled differently.
pub fn check_name_spellings(
    names: impl IntoIterator<Item = ryo_relations::domain::relations::types::Name>,
    symbols: &Symbols,
    arenas: &[ArenaBytes],
) -> Result<(), SpellingError> {
    let mut seen: hashbrown::HashMap<TextHash, &[u8], rustc_hash::FxBuildHasher> =
        hashbrown::HashMap::default();
    for name in names {
        let Some(place) = place_of(name, places_of(symbols), symbols) else {
            continue;
        };
        let (start, end) = (
            usize::try_from(place.start.0).unwrap_or(usize::MAX),
            usize::try_from(place.end.0).unwrap_or(usize::MAX),
        );
        let bytes = match place.origin {
            Origin::Derived(worker) if worker == CROSS_FILE => {
                derived_of(symbols).0.get(start..end)
            }
            Origin::Derived(worker) => arenas
                .get(usize::try_from(worker.0).unwrap_or(usize::MAX))
                .and_then(|arena| arena.0.get(start..end)),
            Origin::Source => None,
        };
        let Some(bytes) = bytes else {
            continue;
        };
        let hash = name.hash;
        let first = *seen.entry(hash).or_insert(bytes);
        if first != bytes {
            return Err(SpellingError::EntityInTwoSources {
                first: String::from_utf8_lossy(first).into_owned(),
                second: String::from_utf8_lossy(bytes).into_owned(),
            });
        }
    }
    Ok(())
}

