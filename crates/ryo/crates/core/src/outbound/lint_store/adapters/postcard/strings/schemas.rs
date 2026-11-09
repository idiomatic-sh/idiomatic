//! The strings document of the postcard store: each text once in a table of strings, a row naming it by its place.

use rustc_hash::FxHashMap;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use std::borrow::Cow;

/// A text, by its place in the table of strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct TextSchema(
    pub(in crate::outbound::lint_store::adapters::postcard) u32,
);

/// Writes `bytes` as one run of bytes, not a byte at a time: the same bytes in the file, written at once.
pub(in crate::outbound::lint_store::adapters::postcard) fn as_bytes<S>(
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(bytes)
}

/// The table of strings being written: each text once, by its place.
#[derive(Default)]
pub(in crate::outbound::lint_store::adapters::postcard) struct StringsSchema<'a> {
    /// The texts, in the order they were first named.
    pub(in crate::outbound::lint_store::adapters::postcard) texts: Vec<Cow<'a, str>>,
    /// The place of each text.
    pub(in crate::outbound::lint_store::adapters::postcard) places: FxHashMap<&'a str, u32>,
}

/// The place of `text` in `strings`, which gains it when it is not there.
pub(in crate::outbound::lint_store::adapters::postcard) fn text<'a>(
    strings: &mut StringsSchema<'a>,
    text: &'a str,
) -> TextSchema {
    if let Some(place) = strings.places.get(text) {
        return TextSchema(*place);
    }
    let place = u32::try_from(strings.texts.len()).unwrap_or(u32::MAX);
    strings.texts.push(Cow::Borrowed(text));
    let _: Option<u32> = strings.places.insert(text, place);
    TextSchema(place)
}

/// The text a row names; a refusal when the table of strings has none there.
pub(in crate::outbound::lint_store::adapters::postcard) fn named(
    strings: &[Cow<'_, str>],
    at: TextSchema,
) -> Result<Box<str>, postcard::Error> {
    strings
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .map(|each| Box::from(&**each))
        .ok_or(postcard::Error::DeserializeBadEncoding)
}

/// A file a row names, by its place among `files` files; a refusal when there is none there.
pub(in crate::outbound::lint_store::adapters::postcard) fn file_number(
    files: usize,
    at: u32,
) -> Result<StoredFileNumber, postcard::Error> {
    if usize::try_from(at).is_ok_and(|place| place < files) {
        Ok(StoredFileNumber(at))
    } else {
        Err(postcard::Error::DeserializeBadEncoding)
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
