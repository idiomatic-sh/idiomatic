//! The edges of the standard syntax tree, in source order.

use crate::domain::syntax::service::parent;
use ryo::languages;

/// Reads the `syntax-child` rows of one TypeScript file from its syntax tree.
pub(crate) struct SyntaxChildItem;

ryo::reads! {
    SyntaxChildItem => languages::SyntaxChild |tree, at, _symbols| {
        kinds: every,
        require (parent(tree, at) != at);
        row {
            parent: node(parent(tree, at)), child: node(at),
            ordinal: value(languages::Integer(i64::from(languages::ordinal_at(tree.elements, at).0))),
            field: value(languages::field_name_at(tree.elements, at)), file: file,
        }
    }
}
