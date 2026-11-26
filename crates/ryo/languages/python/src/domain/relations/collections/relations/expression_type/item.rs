//! Read one classification row per original expression and union member.
use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::model::names::service::fixed_word;
use crate::domain::model::names::types::FixedWords;
use crate::domain::model::view::types::PartState;
use crate::domain::node::service::{keyed_place, node_cell, span_at, table_id};
use crate::domain::node::types::SyntaxFile;
use crate::domain::syntax::service::{category_of, kind_of};
use crate::domain::type_facts::service::resolutions;
use crate::domain::{expression_types::{types::{Resolution}}, nodes::types::{Category}};
use ryo::languages::{self, node_table};

/// The reason incomplete parser payloads cannot establish basic types.
use super::constants::INCOMPLETE;
/// Reads inferred expression classifications on demand.
pub(crate) struct ExpressionTypeItem;

/// Says the relation is read only in part when the parser did not read the whole file: a type may be missing.
fn told_incomplete(
    file: &PythonFile<'_>,
    rows: &mut dyn languages::Rows<languages::ExpressionTypeRow>,
) {
    if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
        rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
    }
}

/// Writes the rows of the node `id`, whose basic type is `found`: one for each type it may be; none for a node that is no
/// expression.
fn type_rows(
    file: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    memo: &mut FixedWords,
    id: languages::Position,
    found: &Resolution,
    rows: &mut dyn languages::Rows<languages::ExpressionTypeRow>,
) {
    if category_of(kind_of(node_table::kind_of(file.table, id))) != Category::Expression {
        return;
    }
    let mut write = |resolution: languages::NameText<'static>,
                     kind: languages::NameText<'static>| {
        rows.write(languages::ExpressionTypeRow {
            id: node_cell(file, id),
            resolution: fixed_word(words, memo, resolution),
            type_: fixed_word(words, memo, kind),
            file: file.file,
            span: span_at(file, id),
        });
    };
    let name = crate::domain::expression_types::constants::name;
    let (label, none) = (
        languages::NameText,
        languages::NameText(super::constants::NO_TYPE),
    );
    match found {
        Resolution::Atom(kind) => write(label(super::constants::ATOM), name(*kind)),
        Resolution::Union(kinds) => {
            for kind in kinds {
                write(label(super::constants::UNION), name(*kind));
            }
        }
        Resolution::Unknown => write(label(super::constants::UNKNOWN), none),
        Resolution::TypeError => write(label(super::constants::TYPE_ERROR), none),
    }
}

/// Finds the basic type of the file's nodes, when no fill of the file has yet: an operation's type is read from its operands',
/// so the types are found whole, once, by the first fill that asks, whole or by key.
fn typed(file: &mut PythonFile<'_>) {
    if file.analysed.types == PartState::Pending {
        resolutions(&file.syntax, &mut file.buffers.types);
        file.analysed.types = PartState::Ready;
    }
}

// The engine fills `expression-type` from the walked file: every row of the file, or the rows of one expression, read from the
// type the file's types give it.
ryo::fills! {
    ExpressionTypeItem => languages::ExpressionType, walk(PythonWalk) |file, words, rows| {
        told_incomplete(file, rows);
        typed(file);
        let memo = &mut file.model.words.get_mut().fixed;
        for (place, found) in file.buffers.types.iter().enumerate() {
            let place = languages::Position(u32::try_from(place).unwrap_or(u32::MAX));
            let id = table_id(file.syntax.tree.segments, place);
            type_rows(&file.syntax, words, memo, id, found, rows);
        }
    }
    key(languages::Column::ExpressionType(languages::ExpressionTypeColumn::Id)) |file, words, key, rows| {
        told_incomplete(file, rows);
        typed(file);
        let Some(place) = keyed_place(file.syntax.file, file.syntax.tree, key) else {
            return;
        };
        if let Some(found) = file.buffers.types.get(usize::from(place)) {
            let id = table_id(file.syntax.tree.segments, place);
            type_rows(&file.syntax, words, &mut file.model.words.get_mut().fixed, id, found, rows);
        }
    }
}
