//! The service of the `table_words` concept: free functions named by verb that write tables as
//! words, so they can wait in a bucket of the store, and read them back, with no IO and no ports.

use crate::domain::rows::types::{RowCount, Table};
use crate::domain::stored_facts::types::PackedWords;
use crate::domain::table_words::types::{DecodeMode, Framing, PackedOffset, PackedSlice, WordSlice};
use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use ryo_relations::domain::relations::types::{
    Bits, CROSS_PLACE, Integer, Name, NamePlace, PlaceNumber, Value, Words,
};











