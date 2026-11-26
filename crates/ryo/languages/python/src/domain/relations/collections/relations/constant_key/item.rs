//! Read supported constant-expression equivalence classes.

use crate::domain::constant_keys::types::{KeyClass, KeyWord};
use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::key_facts::service::keyed;
use crate::domain::key_facts::types::KeyBuild;
use crate::domain::model::view::types::PartState;
use crate::domain::node::service::{keyed_place, node_cell, span_at, table_id};
use crate::domain::node::types::SyntaxFile;
use crate::domain::{
    constant_keys::{
        constants::INCOMPLETE,
        service::{key_of, prepare},
    },
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::service::kind_bits,
    syntax::types::Kind,
};
use ryo::languages::{self, KeyState, RetainedKeyIndex};

/// Reads one source's supported constant-expression keys.
pub(crate) struct ConstantKeyItem;
/// The retained key reader's completeness state.
fn state(index: &RetainedKeyIndex<KeyClass, KeyWord>) -> KeyState {
    index.state
}

ryo::reads! {
    ConstantKeyItem => languages::ConstantKey, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(tree, &model.constant_keys, symbols, |at, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols) == Visibility::Visible).into());
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            given key = key_of(&index, expression)?;
            row { id: node(expression), key: node(key), file: file, span: span(expression), }
        }
        when state(&index) {
            KeyState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            KeyState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            KeyState::Complete => {}
        }
    }
}

/// Says the relation is read only in part when the parser did not read the whole file: a constant may be missing.
fn told_incomplete(
    file: &PythonFile<'_>,
    rows: &mut dyn languages::Rows<languages::ConstantKeyRow>,
) {
    if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
        rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
    }
}

/// The row of the constant `id`, whose value's class is `class`.
fn key_row(
    file: &SyntaxFile<'_>,
    id: languages::Position,
    class: KeyClass,
) -> languages::ConstantKeyRow {
    let (id, key) = (table_id(file.tree.segments, id), table_id(file.tree.segments, class.representative));
    languages::ConstantKeyRow {
        id: node_cell(file, id),
        key: node_cell(file, key),
        file: file.file,
        span: span_at(file, id),
    }
}

/// Keys the constants of the file, when no fill of the file has yet: a value's class is named by a member of it, so the
/// constants are keyed whole, once, by the first fill that asks, whole or by key.
pub(crate) fn keyed_constants(file: &mut PythonFile<'_>, words: &mut languages::FileWords) {
    if file.analysed.keys == PartState::Pending {
        let index = &mut file.buffers.keys;
        keyed(
            &file.syntax,
            &mut KeyBuild {
                index,
                words,
                text: &mut file.buffers.text,
            },
        );
        file.analysed.keys = PartState::Ready;
    }
}

// The engine fills `constant-key` from the walked file: every row of the file, or the row of one constant, read from the class
// the file's keys give it.
ryo::fills! {
    ConstantKeyItem => languages::ConstantKey, walk(PythonWalk) |file, words, rows| {
        keyed_constants(file, words);
        let classes = file.buffers.keys.classes.iter();
        for (place, class) in classes.enumerate() {
            if let Some(class) = class {
                let place = languages::Position(u32::try_from(place).unwrap_or(u32::MAX));
                rows.write(key_row(&file.syntax, place, *class));
            }
        }
        told_incomplete(file, rows);
    }
    key(languages::Column::ConstantKey(languages::ConstantKeyColumn::Id)) |file, words, key, rows| {
        keyed_constants(file, words);
        let found = keyed_place(file.syntax.file, file.syntax.tree, key)
            .and_then(|place| Some((place, file.buffers.keys.classes.get(usize::from(place)).copied().flatten()?)));
        if let Some((place, class)) = found {
            rows.write(key_row(&file.syntax, place, class));
        }
        told_incomplete(file, rows);
    }
}
