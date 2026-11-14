//! The `reference` relation of the language: a path used where it is not declared, by the name it writes.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::references::service::{is_written_use, name};
use crate::domain::references::types::Usage;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `reference` relation's rows to.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Path)] as kind_bits,
        expanded,
        require is_written_use(tree, at) == Usage::Written;
        given target = name(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}
