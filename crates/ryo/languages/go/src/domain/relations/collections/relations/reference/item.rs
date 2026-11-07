//! The `reference` relation of the language: a name used where it is not declared, and a dotted name used, by the whole dotted text:
//! `a.b.c` is one reference, not also `a`. The names a declaration writes are no reference, and neither is the name after a dot or a
//! label. The key of a composite literal is a field or a variable, which the language cannot tell apart without types: its target is the
//! unknown value of an ambiguous key.

use crate::domain::names::service::{dotted, unknown};
use crate::domain::names::types::Unresolved;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::slots::service::referencing;
use crate::domain::slots::types::Referencing;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `reference` relation to, which reads its rows from a file's tree.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference |tree, at, symbols| {
        kinds: [Kind::Node(Node::Ident), Kind::Node(Node::SelectorExpr)] as kind_bits,
        when referencing(tree, at) {
            Referencing::Used => {
                given target = dotted(tree, symbols, at)?;
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(target),
                    file: file,
                    span: span(at),
                }
            }
            Referencing::Unclear => {
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(unknown(Unresolved::AmbiguousKey)),
                    file: file,
                    span: span(at),
                }
            }
            Referencing::Other => {}
        }
    }
}
