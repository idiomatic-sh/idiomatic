//! The `scope` relation of the language: the module of a file, and a function, a class, a lambda or a comprehension, each under the
//! scope that encloses it, the module's for the outermost; the module encloses itself.

use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::scope_owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `scope` rows of one Python file from its syntax tree.
pub(crate) struct ScopeItem;

ryo::reads! {
    ScopeItem => languages::Scope, model(Declarations) |tree, model, at, _symbols| {
        kinds: [
            Kind::Node(Node::Module),
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::ClassDef),
            Kind::Node(Node::Lambda),
            Kind::Node(Node::ListComp),
            Kind::Node(Node::SetComp),
            Kind::Node(Node::DictComp),
            Kind::Node(Node::GeneratorExp),
        ] as kind_bits,
        row { parent: value(scope_owner(tree, at)), child: node(at), file: file, span: span(at) }
    }
}
