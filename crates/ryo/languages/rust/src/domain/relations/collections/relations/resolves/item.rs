//! The `resolves` relation of the language: each type written as a path of one name, with the declaration, import or type parameter
//! that binds the name where it is written.

use crate::domain::nodes::types::RustNode;
use crate::domain::resolutions::service::{binding_of, holder, plain_type_name};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `resolves` relation's rows to.
pub(crate) struct ResolvesItem;

ryo::reads! {
    ResolvesItem => languages::Resolves |tree, at, symbols| {
        kinds: [Kind::Node(RustNode::PathType)] as kind_bits,
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
