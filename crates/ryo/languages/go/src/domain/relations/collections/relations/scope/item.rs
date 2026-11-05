//! The `scope` relation of the language: the file, and each function, method and function literal, each under the scope that
//! encloses it, the file's for the outermost; the file encloses itself. A `go.mod` is a file of the module too, and encloses itself.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::GoNode;
use crate::domain::scopes::service::scope_owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `scope` relation to, which reads its rows from a file's tree.
pub(crate) struct ScopeItem;

ryo::reads! {
    ScopeItem => languages::Scope |tree, at, _symbols| {
        kinds: [Kind::Node(GoNode::File), Kind::Extra(Extra::ModFile), Kind::Node(GoNode::FuncDecl), Kind::Node(GoNode::FuncLit)] as kind_bits,
        row { parent: value(scope_owner(tree, at)), child: node(at), file: file, span: span(at) }
    }
}
