//! The `completion` service: free functions named by verb that finish the table of a file once it is built: the names of its kinds and roles, and the notes only the language reads, with no IO and no ports.

use crate::domain::elements::service::{engine, engine_mut, positions};
use crate::domain::elements::types::{Completion, CompletionEngine, Elements, KindSum, Tree};
use ryo_relations::domain::relations::constants::{NO_FIELD, NO_SYMBOL};
use ryo_relations::domain::relations::types::{Bytes, FieldId, FileId, KindBits, Position};
use crate::domain::symbols::service::{hash_in, intern_derived, vocabulary_symbol};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::terms::types::NameText;

/// What a kind's number and name are mixed with before they are added to the vocabulary.
const MIX: u64 = 0x9E37_79B9_7F4A_7C15;

/// The completion of the table of the file whose bytes are `source`, for the language to finish it through: its decoded node values are
/// interned first.
pub fn completion<'a>(
    elements: &'a mut Elements,
    symbols: &'a mut Symbols,
    source: ByteSlice<'a>,
    file: FileId,
) -> Completion<'a> {
    intern_decoded(elements, symbols, source);
    Completion::from(CompletionEngine {
        elements,
        symbols,
        source,
        file,
    })
}

/// Interns the decoded value of each node, after every token of the file's bytes, and gives the node that text.
pub fn intern_decoded(elements: &mut Elements, symbols: &mut Symbols, source: ByteSlice<'_>) {
    intern_decoded_from(elements, symbols, source, 0);
}

/// Intern only decoded entries added after an append, preserving every previous symbol.
pub(crate) fn intern_decoded_from(
    elements: &mut Elements,
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    from: usize,
) {
    // Source tokens have all been interned; derived node values must follow them.
    let count = engine(elements).decoded.len();
    for at in from..count {
        #[cfg(test)]
        {
            engine_mut(elements).decoded_intern_visits += 1;
        }
        let table = engine(elements);
        let Some(decoded) = table.decoded.get(at).copied() else {
            break;
        };
        if decoded.property.0 != 0
            || crate::domain::elements::service::flags_at(elements, decoded.element).0
                & ryo_relations::domain::relations::constants::TOKEN.0
                != 0
        {
            continue;
        }
        let from = usize::try_from(decoded.from.0).unwrap_or(usize::MAX);
        let to = usize::try_from(decoded.to.0).unwrap_or(usize::MAX);
        let symbol = table
            .decoded_text
            .0
            .get(from..to)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // ignore: idiomatic.arch/no-ok-on-result - the bytes are cut from text at the places the text was written: a cut that is no text has no text, which the caller reads as no value
            .map(|value| intern_derived(symbols, source, NameText(value)));
        if let Some(symbol) = symbol {
            *crate::domain::elements::service::number_mut(&mut elements.text, decoded.element) =
                symbol;
        }
    }
}

/// Names the kind numbered `kind` for the file, once the whole table is built (a name that is not in
/// the file's bytes goes after the names that are), so the table says what each element is called.
pub fn name_kind(completion: &mut Completion<'_>, kind: KindBits, name: NameText<'_>) {
    let at = usize::from(kind.0);
    let completion = completion.as_mut();
    let symbol = vocabulary_symbol(completion.symbols, completion.source, kind, name);
    let named = hash_in(completion.symbols, symbol).map_or(0, |hash| hash.0);
    let table = engine_mut(completion.elements);
    if table.names.len() <= at {
        table.names.resize(at + 1, NO_SYMBOL);
    }
    if let Some(slot) = table.names.get_mut(at) {
        *slot = symbol;
    }
    table.vocabulary = KindSum(
        table
            .vocabulary
            .0
            .wrapping_add((named ^ u64::from(kind.0)).wrapping_mul(MIX)),
    );
}

/// Writes the notes of the file, the bytes only the language reads: `write` is given the tree that was built, to read, and the room for the
/// notes. The tree is not changed.
pub fn write_notes(completion: &mut Completion<'_>, write: impl FnOnce(&Tree<'_>, &mut Bytes)) {
    let completion = completion.as_mut();
    let table = engine_mut(completion.elements);
    let mut notes = std::mem::take(&mut table.notes);
    let tree = Tree {
        elements: completion.elements,
        source: completion.source,
        file: completion.file,
    };
    write(&tree, &mut notes);
    engine_mut(completion.elements).notes = notes;
}

/// Names the role numbered `field` for the file, once the whole table is built, so the table says what each role is called.
pub fn name_field(completion: &mut Completion<'_>, field: FieldId, name: NameText<'_>) {
    let at = usize::from(field.0);
    let completion = completion.as_mut();
    let symbol = intern_derived(completion.symbols, completion.source, name);
    let table = engine_mut(completion.elements);
    if table.field_names.len() <= at {
        table.field_names.resize(at + 1, NO_SYMBOL);
    }
    if let Some(slot) = table.field_names.get_mut(at) {
        *slot = symbol;
    }
}

/// Gives each element of the file's own tree its role: `assign` is given the tree that was built, to read, and the position, and answers
/// the role of the element under its parent. The tree is not changed while it is read.
pub fn write_fields(
    completion: &mut Completion<'_>,
    assign: impl Fn(&Tree<'_>, Position) -> FieldId,
) {
    let completion = completion.as_mut();
    let mut roles = std::mem::take(&mut completion.elements.field);
    let column = roles.as_mut();
    column.values.clear();
    column
        .values
        .resize(completion.elements.kind.as_ref().values.len(), NO_FIELD);
    let tree = Tree {
        elements: completion.elements,
        source: completion.source,
        file: completion.file,
    };
    for at in positions(completion.elements).chain(
        crate::domain::elements::service::expanded_positions(completion.elements),
    ) {
        *crate::domain::elements::service::number_mut(&mut roles, at) = assign(&tree, at);
    }
    completion.elements.field = roles;
}
