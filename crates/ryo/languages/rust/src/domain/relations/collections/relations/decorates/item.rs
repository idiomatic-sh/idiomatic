//! The `decorates` relation of the language: an attribute of the declaration it is on, by the path it names and the arguments it passes, in its place.

use crate::domain::nodes::types::RustNode;
use crate::domain::references::service::{
    attribute_arguments, attribute_ordinal, attribute_target,
};
use crate::domain::scopes::service::is_nameless_function;
use crate::domain::scopes::types::Namelessness;
use crate::domain::syntax::service::{kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `decorates` relation's rows to.
pub(crate) struct DecoratesItem;

ryo::reads! {
    DecoratesItem => languages::Decorates |tree, at, symbols| {
        kinds: [Kind::Node(RustNode::Attr)] as kind_bits,
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
