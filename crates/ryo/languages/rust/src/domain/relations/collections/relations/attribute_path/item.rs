//! The `attribute_path` relation of the language: every path an attribute writes, a meta's path or a path written in an attribute's
//! token tree, as the name it is written as.

use crate::domain::attribute_paths::service::{path_name, written_paths};
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{end, kind_bits, start};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `attribute_path` relation's rows to.
pub(crate) struct AttributePathItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    AttributePathItem => languages::AttributePath, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Path), Kind::Node(Node::TokenTree)] as kind_bits,
        each (written_paths(tree, at)) as path {
            row {
                id: node(path.first),
                name: value(path_name(tree, symbols, path)),
                file: file,
                span: value(languages::Span { start: start(tree, path.first), end: end(tree, path.last) }),
            }
        }
    }
}
