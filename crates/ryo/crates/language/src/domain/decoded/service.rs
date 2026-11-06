//! The `decoded` service: free functions named by verb that read the text the language's library decoded for an element, with no IO
//! and no ports.

use crate::domain::elements::service::engine;
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{Position, PropertyId};
use ryo_relations::domain::terms::types::NameText;

/// The text the token at `at` stands for, when the language's library decoded it.
#[must_use]
pub fn decoded_at(elements: &Elements, at: Position) -> Option<NameText<'_>> {
    property_at(elements, at, PropertyId(0))
}

/// A scalar parser property stored on this node, without claiming a source location.
#[must_use]
pub fn property_at(
    elements: &Elements,
    at: Position,
    property: PropertyId,
) -> Option<NameText<'_>> {
    let table = engine(elements);
    let Ok(found) = table
        .decoded
        .binary_search_by_key(&(at, property), |each| (each.element, each.property))
    else {
        return None; // a binary search answers where a key it does not find would go: that is the key's absence, not a failure
    };
    let each = table.decoded.get(found)?;
    table
        .decoded_text
        .0
        .get(
            usize::try_from(each.from.0).unwrap_or(usize::MAX)
                ..usize::try_from(each.to.0).unwrap_or(usize::MAX),
        )
        // The bytes are cut from text at the places the text was written: a cut that is no text has no text, which the caller reads as no value.
        .and_then(|bytes| match std::str::from_utf8(bytes) {
            Ok(text) => Some(text),
            Err(_) => None,
        })
        .map(NameText)
}
