//! The `static_call` relation of the language: every call, by the dotted name it calls; a callee that is no dotted name has the
//! unknown value of a dynamic callee.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::dotted_or;
use crate::domain::names::types::Unresolved;
use crate::domain::nodes::types::Category;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{nth, owner};
use crate::domain::scopes::types::CategoryIndex;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `static-call` rows of one Python file from its syntax tree.
pub(crate) struct StaticCallItem;

ryo::reads! {
    StaticCallItem => languages::StaticCall, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Call)] as kind_bits,
        given callee = nth(tree, at, Category::Expression, CategoryIndex(0))?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            target: value(dotted_or(tree, symbols, callee, Unresolved::DynamicCallee)),
            file: file,
            span: span(callee),
        }
    }
}
