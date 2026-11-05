//! The `defines` relation of the language: every name a file declares or brings in, its refinement under its shared kind.

use crate::domain::bindings::service::names_a_possible_constant;
use crate::domain::bindings::types::PossibleConstant;
use crate::domain::declarations::service::{
    declaration, declaration_kind, declaration_span, identity, refined,
};
use crate::domain::nodes::types::RustNode;
use crate::domain::relations::constants::{NAME_LIKE_CONSTANT, VARIABLE};
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `defines` relation's rows to.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines |tree, at, symbols| {
        kinds: [
            Kind::Node(RustNode::Struct),
            Kind::Node(RustNode::Enum),
            Kind::Node(RustNode::Variant),
            Kind::Node(RustNode::RecordField),
            Kind::Node(RustNode::Trait),
            Kind::Node(RustNode::Const),
            Kind::Node(RustNode::Static),
            Kind::Node(RustNode::Module),
            Kind::Node(RustNode::TypeAlias),
            Kind::Node(RustNode::Union),
            Kind::Node(RustNode::MacroRules),
            Kind::Node(RustNode::MacroDef),
            Kind::Node(RustNode::TypeParam),
            Kind::Node(RustNode::ConstParam),
            Kind::Node(RustNode::LetStmt),
            Kind::Node(RustNode::IdentPat),
            Kind::Node(RustNode::Fn),
            Kind::Node(RustNode::TupleField),
            Kind::Node(RustNode::LifetimeParam),
            Kind::Node(RustNode::UseTree),
            Kind::Node(RustNode::ExternCrate),
            Kind::Node(RustNode::Impl),
            Kind::Node(RustNode::Label),
        ] as kind_bits,
        when declaration(tree, symbols, at) {
            Some(declared) => {
                row {
                    id: node(identity(&declared, at)),
                    kind: value(declaration_kind(declared.declared)),
                    refinement: value(refined(tree, symbols, &declared)),
                    name: value(declared.name),
                    owner: value(owner(tree, at)),
                    file: file,
                    span: value(declaration_span(&declared)),
                }
            }
            None => {
                when names_a_possible_constant(tree, at) {
                    PossibleConstant::Possible => {
                        partial NAME_LIKE_CONSTANT => languages::Missing::Kind(VARIABLE);
                    }
                    PossibleConstant::Impossible => {}
                }
            }
        }
    }
}
