//! The test support of the `completion` concept: the completion calls the tests of other crates make to finish an element table.

use super::service;
use crate::domain::elements::types::{Completion, Elements};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits};
use ryo_relations::domain::terms::types::NameText;

/// Opens the completion of the element table of a file.
pub fn completion<'a>(
    elements: &'a mut Elements,
    symbols: &'a mut Symbols,
    source: ByteSlice<'a>,
    file: FileId,
) -> Completion<'a> {
    service::completion(elements, symbols, source, file)
}

/// Names the nodes of a kind.
pub fn name_kind(completion: &mut Completion<'_>, kind: KindBits, name: NameText<'_>) {
    service::name_kind(completion, kind, name);
}
