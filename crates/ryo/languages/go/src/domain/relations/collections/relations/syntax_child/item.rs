//! The `syntax_child` relation of the language: every edge of the tree, ordered under its parent. The library's tree names no roles
//! the language keeps, so a child's role is the empty name.

use crate::domain::syntax::service::{is_root, ordinal, parent};
use crate::domain::syntax::types::Rootness;
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_child` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxChildItem;

ryo::reads! {
    SyntaxChildItem => languages::SyntaxChild |tree, at, _symbols| {
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
