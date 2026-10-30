//! The `function` relation of the language: every `def`, `async` or not, at any depth, methods included; a lambda is no function.

use crate::domain::extras::types::Extra;
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `function` rows of one Python file from its syntax tree.
pub(crate) struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function, model(Declarations) |tree, model, at, _symbols| {
        kinds: [Kind::Node(Node::FunctionDef)] as kind_bits,
        given identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
        given name = name_of(tree, identifier)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}
