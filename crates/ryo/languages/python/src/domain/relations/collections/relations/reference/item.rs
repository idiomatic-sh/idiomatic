//! The `reference` relation of the language: a name read or deleted, and a dotted name read, by the whole dotted text: `a.b.c` is one
//! reference, not also `a`. A name bound is no reference; in `obj.x = 1` the object is.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::{dotted, use_of};
use crate::domain::names::types::Use;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `reference` rows of one Python file from its syntax tree.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Name), Kind::Node(Node::Attribute)] as kind_bits,
        require use_of(tree, at) == Use::Reference;
        given target = dotted(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}
