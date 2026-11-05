//! The `static_call` relation of the language: every call, by the dotted name it calls; a callee that is no dotted name has the unknown
//! value of a dynamic callee. A call of a type written out (`[]byte(s)`) converts, and has no row.

use crate::domain::names::service::{callee_of, dotted, unknown};
use crate::domain::names::types::{Callee, Unresolved};
use crate::domain::nodes::types::GoNode;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::{kind_bits, nodes};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `static_call` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticCallItem;

ryo::reads! {
    StaticCallItem => languages::StaticCall |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::CallExpr)] as kind_bits,
        given callee = nodes(tree, at).next()?;
        when callee_of(tree, callee) {
            Callee::Named => {
                given target = dotted(tree, symbols, callee)?;
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(target),
                    file: file,
                    span: span(callee),
                }
            }
            Callee::Dynamic => {
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(unknown(Unresolved::DynamicCallee)),
                    file: file,
                    span: span(callee),
                }
            }
            Callee::Conversion => {}
        }
    }
}
