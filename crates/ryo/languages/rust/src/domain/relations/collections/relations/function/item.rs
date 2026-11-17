//! The `function` relation of the language: every `fn` item with a name, free or associated, nested or not, is a row.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, named};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `function` relation's rows to.
pub(crate) struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function, model(ReadIndexes) |tree, _model, at, _symbols| {
        kinds: [Kind::Node(Node::Fn)] as kind_bits,
        given name = named(tree, at)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}
