//! The `resolves` relation: lexical name reads associated with their local declaration.

use crate::domain::bindings::service::resolve;
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{context, owner};
use crate::domain::scopes::types::Context;
use crate::domain::syntax::service::{kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `resolves` rows of one Python file from its syntax tree.
pub(crate) struct ResolvesItem;

ryo::reads! {
    ResolvesItem => languages::Resolves, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Name)] as kind_bits,
        require context(tree, at) == Context::Load;
        given name = name_of(tree, at)?;
        given definition = resolve(tree, model, at, name)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            name: value(name),
            definition: node(definition),
            file: file,
            span: span(at),
        }
    }
}
