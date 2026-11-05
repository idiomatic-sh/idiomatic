//! The `function` relation of the language: every `func` declaration at any depth, methods included; a function literal is no function.

use crate::domain::names::service::name_of;
use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::signature;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `function` relation to, which reads its rows from a file's tree.
pub(crate) struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [Kind::Node(Node::FuncDecl)] as kind_bits,
        given identifier = signature(tree, at).name?;
        given name = name_of(tree, identifier)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}
