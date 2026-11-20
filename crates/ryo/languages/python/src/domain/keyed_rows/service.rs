//! The `keyed_rows` service: free functions named by verb that write, of five relations the semantic model answers, the rows
//! that hold one key and no other row, with no IO and no ports. Each reads the part of the model its relation's whole filler
//! reads, or less, and looks the key up there: none walks the file's nodes, its reads or its shadows. A row is built by the
//! function that builds it for the whole relation, so the rows of every key together are the whole relation's rows.

use crate::domain::binding_facts::service::{flow_read, read_row, shadow_row};
use crate::domain::lexical_facts::service::{own_count, reference_row, resolves_row, scope_row};
use crate::domain::lexical_facts::types::QualifiedPath;
use crate::domain::model::flow::service::listed;
use crate::domain::model::names::service::find;
use crate::domain::model::names::types::DottedText;
use crate::domain::model::resolution::service::resolve;
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use crate::domain::node::service::keyed_place;
use ryo::languages::{
    BindingReadRow, BindingShadowRow, CellWord, FileWords, Position, ReferenceRow, ResolvesRow,
    Rows, ScopeRow, facts,
};

/// The node of the file's own tree a key cell names, by its place; none for a cell that names no node of it. The relations
/// that state the file's own names have no row on a node of an embedded expression.
fn own_place(read: ModelRead<'_>, key: CellWord) -> Option<Position> {
    keyed_place(read.file, read.tree, key).filter(|at| *at < own_count(read))
}

/// Writes the row of the scope `key` names: one read of the scope's column.
pub(crate) fn scope_rows_of<V: ModelView>(
    file: &mut V,
    key: CellWord,
    rows: &mut dyn Rows<ScopeRow>,
) {
    let read = file.semantics(ModelPart::Lexical);
    if let Some(row) = own_place(read, key).and_then(|at| scope_row(read, at)) {
        rows.write(row);
    }
}

/// Writes the row of the name or attribute expression `key` names, when it is read or deleted and is not the object of an
/// attribute that is read: the chain of attributes under that one node.
pub(crate) fn reference_rows_of<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    key: CellWord,
    rows: &mut dyn Rows<ReferenceRow>,
) {
    let read = file.semantics(ModelPart::Lexical);
    let Some(at) = own_place(read, key) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    let mut path = QualifiedPath(std::mem::take(&mut known.path.0));
    if let Some(row) = reference_row(read, words, &mut path, at) {
        rows.write(row);
    }
    known.path = DottedText(path.0);
}

/// Writes the row of the name expression `key` names, when it is read and resolves to a definition of the file: the static
/// rule asked of that one name, over the lexical part alone.
pub(crate) fn resolves_rows_of<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    key: CellWord,
    rows: &mut dyn Rows<ResolvesRow>,
) {
    let read = file.semantics(ModelPart::Lexical);
    let Some(at) = own_place(read, key) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    let resolved = |name| {
        resolve(
            &read.model.definitions,
            &read.model.lexical,
            read.tree,
            at,
            name,
        )
    };
    if let Some(row) = resolves_row(read, words, &mut known, at, resolved) {
        rows.write(row);
    }
}

/// Writes the rows of the reads of the name whose word is `key`, in evaluation order: the name is looked up among the file's
/// names by its text, and its reads in the list of the reads by name.
pub(crate) fn binding_read_rows_of<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    key: CellWord,
    rows: &mut dyn Rows<BindingReadRow>,
) {
    let Some(read) = flow_read(file, ModelPart::ReadsByName, rows) else {
        return;
    };
    let Some(name) = find(&read.model.names, facts::text(words, key)) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for at in listed(&read.model.flow.reads_by_name, name) {
        let Some(each) = read.model.flow.reads.get(at.0) else {
            continue;
        };
        let row = read_row(read, words, &mut known, each);
        // A word that is no text of these words reads as the empty text: only a row that holds the key is the key's.
        if u64::from(row.name.0) == key.0 {
            rows.write(row);
        }
    }
}

/// Writes the rows of the shadows of the binding `key` names, in evaluation order: the binding is looked up in the list of the
/// shadows by binding.
pub(crate) fn binding_shadow_rows_of<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    key: CellWord,
    rows: &mut dyn Rows<BindingShadowRow>,
) {
    let Some(read) = flow_read(file, ModelPart::ShadowsByBinding, rows) else {
        return;
    };
    let Some(binding) = keyed_place(read.file, read.tree, key) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for at in listed(&read.model.flow.shadows_by_binding, binding) {
        if let Some(each) = read.model.flow.shadows.get(at.0) {
            rows.write(shadow_row(read, words, &mut known, each));
        }
    }
}
