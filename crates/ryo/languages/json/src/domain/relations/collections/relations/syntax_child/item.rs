//! The `syntax_child` relation of the language: every edge of the tree, ordered under its parent.

use crate::domain::model::types::Model;
use crate::domain::syntax::service::{is_root, ordinal, parent};
use crate::domain::syntax::types::Rootness;
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_child` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxChildItem;

ryo::reads! {
    SyntaxChildItem => languages::SyntaxChild, model(Model) |tree, model, at, _symbols| {
        kinds: every,
        require (is_root(tree, at) == Rootness::Inner);
        row {
            parent: node(parent(tree, at)),
            child: node(at),
            ordinal: value(ordinal(tree, at)),
            field: value(languages::field_name_at(tree.elements, at)),
            file: file,
        }
    }
}
