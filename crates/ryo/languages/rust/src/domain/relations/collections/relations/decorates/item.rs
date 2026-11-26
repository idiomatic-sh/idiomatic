//! The `decorates` relation of the language: an attribute of the declaration it is on, by the path it names and the arguments it passes, in its place.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::references::service::{
    attribute_arguments, attribute_ordinal, attribute_target,
};
use crate::domain::scopes::service::is_nameless_function;
use crate::domain::scopes::types::Namelessness;
use crate::domain::syntax::service::{kind_bits, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `decorates` relation's rows to.
pub(crate) struct DecoratesItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    DecoratesItem => languages::Decorates, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Attr)] as kind_bits,
        given declaration = parent(tree, at);
        require is_nameless_function(tree, declaration) == Namelessness::Named;
        given target = attribute_target(tree, symbols, at)?;
        row {
            occurrence: node(at),
            declaration: node(declaration),
            ordinal: value(languages::Integer(i64::from(attribute_ordinal(tree, declaration, at).0))),
            target: value(target),
            arguments: value(attribute_arguments(tree, symbols, at)),
            file: file,
            span: span(at),
        }
    }
}
