//! The `module_paths` service: free functions named by verb that read a module path, and an `imports` item's module
//! pattern, as the matching port reads a path, with no IO and no ports. A path's segments are its names, whatever the
//! language writes between them.

use super::constants::{ANY_MODULE, ANY_SEGMENTS, NAME_CHARACTERS, PATH_SEPARATOR};
use crate::domain::imports::types::{
    AllowItem, ModuleName, ModulePattern, ModulePrefix, WrittenPath,
};

/// The names a module path or a pattern is written with, in order: what separates them is any run of characters that
/// are not part of a name; a pattern's `*` is a name of its own.
#[must_use]
pub fn segments(text: &WrittenPath) -> Vec<ModuleName> {
    text.0
        .split(|character| {
            !(char::is_alphanumeric(character)
                || NAME_CHARACTERS.contains(&character)
                || ANY_MODULE.starts_with(character))
        })
        .filter(|segment| !segment.is_empty())
        .map(|segment| ModuleName(segment.into()))
        .collect()
}

/// The module pattern of an `imports` item as the matching port reads it: its segments as a path's, a segment `*`
/// standing for any number of segments, as a module pattern's `*` crosses every separator.
#[must_use]
pub fn pattern(item: &AllowItem) -> ModulePattern {
    ModulePattern(
        segments(&WrittenPath(item.0.clone()))
            .iter()
            .map(|segment| {
                if &*segment.0 == ANY_MODULE {
                    ANY_SEGMENTS
                } else {
                    &segment.0
                }
            })
            .collect::<Vec<_>>()
            .join(PATH_SEPARATOR)
            .into(),
    )
}

/// Each module a written path lies in, the path itself last, as the matching port reads a path: `a`, `a/b`, `a/b/c`.
#[must_use]
pub fn prefixes(written: &WrittenPath) -> Vec<ModulePrefix> {
    let segments = segments(written)
        .into_iter()
        .map(|segment| segment.0)
        .collect::<Vec<_>>();
    (1..=segments.len())
        .map(|length| {
            ModulePrefix(
                segments
                    .get(..length)
                    .unwrap_or_default()
                    .join(PATH_SEPARATOR)
                    .into(),
            )
        })
        .collect()
}
