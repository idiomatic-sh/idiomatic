//! The `spelling` service: whether the names the files handed over are spelled alike, with no IO and no ports.

use super::errors::SpellingError;
use crate::domain::finish::types::Resolving;
use crate::domain::table_model::types::Table;
use ryo_language::domain::relations::types::{CROSS_FILE, Origin, TextHash, Value};
use ryo_language::domain::symbols::service::{derived_of, place_of, places_of};

/// Checks that the names the files handed over are the same entity exactly when they are spelled the same: the cross-file steps
/// compare names by their hash, so two spellings with one hash would join as one.
///
/// # Errors
///
/// `EntityInTwoSources` when two names with one hash are spelled differently.
pub(crate) fn check_spellings<'a>(
    tables: impl IntoIterator<Item = &'a Table>,
    resolving: &Resolving<'_>,
) -> Result<(), SpellingError> {
    let mut seen: hashbrown::HashMap<TextHash, &[u8], rustc_hash::FxBuildHasher> =
        hashbrown::HashMap::default();
    for table in tables {
        for value in &table.cells {
            let Value::Name(name) = value else {
                continue;
            };
            let Some(place) = place_of(*name, places_of(resolving.cross), resolving.cross) else {
                continue;
            };
            let (start, end) = (
                usize::try_from(place.start.0).unwrap_or(usize::MAX),
                usize::try_from(place.end.0).unwrap_or(usize::MAX),
            );
            let bytes = match place.origin {
                Origin::Derived(worker) if worker == CROSS_FILE => {
                    derived_of(resolving.cross).0.get(start..end)
                }
                Origin::Derived(worker) => resolving
                    .arenas
                    .0
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
    }
    Ok(())
}
