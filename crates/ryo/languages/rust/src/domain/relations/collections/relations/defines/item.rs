//! The `defines` relation of the language: every name a file declares or brings in, its refinement under its shared kind.

use crate::domain::bindings::service::names_a_possible_constant;
use crate::domain::bindings::types::PossibleConstant;
use crate::domain::declarations::service::{
    declaration, declaration_kind, declaration_span, identity, refined,
};
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::relations::constants::{NAME_LIKE_CONSTANT, VARIABLE};
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `defines` relation's rows to.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [
            Kind::Node(Node::Struct),
            Kind::Node(Node::Enum),
            Kind::Node(Node::Variant),
            Kind::Node(Node::RecordField),
            Kind::Node(Node::Trait),
            Kind::Node(Node::Const),
            Kind::Node(Node::Static),
            Kind::Node(Node::Module),
            Kind::Node(Node::TypeAlias),
            Kind::Node(Node::Union),
            Kind::Node(Node::MacroRules),
            Kind::Node(Node::MacroDef),
            Kind::Node(Node::TypeParam),
            Kind::Node(Node::ConstParam),
            Kind::Node(Node::LetStmt),
            Kind::Node(Node::IdentPat),
            Kind::Node(Node::Fn),
            Kind::Node(Node::TupleField),
            Kind::Node(Node::LifetimeParam),
            Kind::Node(Node::UseTree),
            Kind::Node(Node::ExternCrate),
            Kind::Node(Node::Impl),
            Kind::Node(Node::Label),
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
