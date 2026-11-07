//! The `syntax_child` relation of the language: every edge of the tree, ordered under its parent. The library's tree names no roles,
//! so a child's role is the empty name.

use crate::domain::model::types::Declarations;
use crate::domain::syntax::service::{is_root, ordinal, parent};
use crate::domain::syntax::types::Rootness;
use ryo::languages;

/// Reads the `syntax-child` rows of one Python file from its syntax tree.
pub(crate) struct SyntaxChildItem;

ryo::reads! {
    SyntaxChildItem => languages::SyntaxChild, model(Declarations) |tree, model, at, _symbols| {
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
