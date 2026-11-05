//! The `inherits` relation of the language: a supertrait a trait requires, and a trait an `impl` implements, for whatever type.

use crate::domain::declarations::service::{implementation, supertraits};
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::service::{has_child, kind, kind_bits};
use crate::domain::syntax::types::{Kind, Offspring};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `inherits` relation's rows to.
pub(crate) struct InheritsItem;

ryo::reads! {
    InheritsItem => languages::Inherits |tree, at, symbols| {
        kinds: [Kind::Node(RustNode::Trait), Kind::Node(RustNode::Impl)] as kind_bits,
        when kind(tree, at) {
            Kind::Node(RustNode::Trait) => {
                require (has_child(tree, at, Kind::Node(RustNode::Name)) == Offspring::Has);
                each (supertraits(tree, symbols, at)) as bound {
                    row {
                        occurrence: node(bound.contract),
                        declaration: node(at),
                        target: value(bound.target),
                        file: file,
                        span: span(bound.contract),
                    }
                }
            }
            _ => {
                given implemented = implementation(tree, symbols, at)?;
                row {
                    occurrence: node(implemented.contract),
                    declaration: node(implemented.declaration),
                    target: value(implemented.target),
                    file: file,
                    span: span(implemented.contract),
                }
            }
        }
    }
}
