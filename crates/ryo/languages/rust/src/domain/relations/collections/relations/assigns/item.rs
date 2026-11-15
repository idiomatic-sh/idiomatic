//! The `assigns` relation of the language: the string literal a single-name `let` binds, linked to the binding and the literal.

use crate::domain::bindings::service::{assigned_string, bound_name};
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `assigns` relation's rows to.
pub(crate) struct AssignsItem;

ryo::reads! {
    AssignsItem => languages::Assigns, model(ReadIndexes) |tree, _model, at, _symbols| {
        kinds: [Kind::Node(Node::LetStmt)] as kind_bits,
        given name = bound_name(tree, at)?;
        given literal = assigned_string(tree, at)?;
        row {
            occurrence: node(literal),
            binding: node(name),
            value: node(literal),
            owner: value(owner(tree, at)),
            file: file,
            span: span(literal),
        }
    }
}
