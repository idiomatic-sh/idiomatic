//! The `scope` relation of the language: the module of a file, the modules, `impl`s, traits and functions in it, each with the scope that
//! encloses it; the module of the file encloses itself.

use crate::domain::nodes::types::RustNode;
use crate::domain::scopes::service::{enclosing_scope, is_nameless_function};
use crate::domain::scopes::types::Namelessness;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `scope` relation's rows to.
pub(crate) struct ScopeItem;

ryo::reads! {
    ScopeItem => languages::Scope |tree, at, _symbols| {
        kinds: [Kind::Node(RustNode::SourceFile), Kind::Node(RustNode::Module), Kind::Node(RustNode::Impl), Kind::Node(RustNode::Trait), Kind::Node(RustNode::Fn)] as kind_bits,
        require is_nameless_function(tree, at) == Namelessness::Named;
        given enclosing = enclosing_scope(tree, at);
        row { parent: node(enclosing), child: node(at), file: file, span: span(at) }
    }
}
