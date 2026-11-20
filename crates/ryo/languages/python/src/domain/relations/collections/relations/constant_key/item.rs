//! Read supported constant-expression equivalence classes.

use crate::domain::constant_keys::types::{KeyClass, KeyWord};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::key_facts::service::keyed;
use crate::domain::key_facts::types::KeyBuild;
use crate::domain::node::service::{node_cell, span_at, typed_nodes};
use crate::domain::node::types::Numbered;
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

// The engine fills `constant-key` from the walked file: every row of the file, once.
ryo::fills! {
    ConstantKeyItem => languages::ConstantKey, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::ConstantKey as languages::CatalogRelation>::Row;
        let index = &mut file.buffers.keys;
        keyed(&file.syntax, &mut KeyBuild { index, words, text: &mut file.buffers.text });
        for (Numbered { id, .. }, class) in typed_nodes(&file.syntax).zip(index.classes.iter()) {
            if let Some(class) = class {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    key: node_cell(&file.syntax, class.representative),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}
