//! The `call_argument` relation of the language: each argument of a call, by its place among the arguments. A call of a type written
//! out converts, and has no row, as it has none of `static_call`.

use crate::domain::names::service::{arguments_of, callee_of, passed};
use crate::domain::names::types::Callee;
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::{kind_bits, nodes};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `call_argument` relation to, which reads its rows from a file's tree.
pub(crate) struct CallArgumentItem;

ryo::reads! {
    CallArgumentItem => languages::CallArgument |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::CallExpr)] as kind_bits,
        given callee = nodes(tree, at).next()?;
        require callee_of(tree, callee) != Callee::Conversion;
        each (arguments_of(tree, at)) as (position, argument) {
            row {
                call: node(at),
                position: value(languages::Integer(i64::from(position.0))),
                expression: node(argument),
                passes: value(passed(tree, argument)),
                file: file,
                span: span(argument),
            }
        }
    }
}
