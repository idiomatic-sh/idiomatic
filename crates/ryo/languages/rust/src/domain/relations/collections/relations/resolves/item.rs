//! The `resolves` relation of the language: each type written as a path of one name, with the declaration, import or type parameter
//! that binds the name where it is written.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::resolutions::service::{binding_of, holder, plain_type_name};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `resolves` relation's rows to.
pub(crate) struct ResolvesItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    ResolvesItem => languages::Resolves, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::PathType)] as kind_bits,
        given name = plain_type_name(tree, at)?;
        given definition = binding_of(tree, at, name)?;
        row {
            occurrence: node(at),
            owner: value(holder(tree, at)),
            name: value(name),
            definition: node(definition),
            file: file,
            span: span(at),
        }
    }
}
