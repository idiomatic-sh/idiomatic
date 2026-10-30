//! The `syntax_child` relation of the language: every edge of the tree, ordered under its parent.

use crate::domain::syntax::service::{is_root, ordinal, parent};
use crate::domain::syntax::types::Rootness;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `syntax_child` relation's rows to.
pub(crate) struct SyntaxChildItem;

ryo::reads! {
    SyntaxChildItem => languages::SyntaxChild |tree, at, _symbols| {
        kinds: every,
        require (is_root(tree, at) == Rootness::Inner);
        row {
            parent: node(parent(tree, at)),
            child: node(at),
            ordinal: value(languages::Integer(i64::from(ordinal(tree, at).0))),
            field: value(languages::field_name_at(tree.elements, at)),
            file: file,
        }
    }
}
