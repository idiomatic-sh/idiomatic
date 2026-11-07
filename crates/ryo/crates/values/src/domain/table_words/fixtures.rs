//! The test support of the `table_words` concept: the table encoding the tests of other crates make.

use super::errors::TableWordsError;
use super::service;
use super::types::WordSlice;
use crate::domain::rows::types::Table;
use crate::domain::stored_facts::types::PackedWords;
use ryo_relations::domain::relations::types::{NamePlace, Words};

/// Packs words into bytes.
#[must_use]
pub fn pack(words: WordSlice<'_>) -> PackedWords {
    service::pack(words)
}

/// Encodes a table into words.
pub fn encode_table(table: &Table, places: &[NamePlace], out: &mut Words) {
    service::encode_table(table, places, out);
}

/// Decodes a whole table from packed words.
pub fn decode_complete(
    packed: &PackedWords,
    places: &mut Vec<NamePlace>,
) -> Result<Table, TableWordsError> {
    service::decode_complete(packed, places)
}
