//! The `numeric_literal` relation of the language: the kind and value of each numeric literal, never its spelling; one in the
//! arguments of a standard macro whose arguments are expressions is read there too. A sign is not part of the literal.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::numeric_literals::{service::stored, types::LiteralNumber};
use crate::domain::syntax::constants::NUMBER_UNREAD;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `numeric_literal` relation's rows to.
pub(crate) struct NumericLiteralItem;

ryo::reads! {
    NumericLiteralItem => languages::NumericLiteral, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Literal)] as kind_bits,
        expanded,
        when stored(tree, at) {
            LiteralNumber::NotNumeric => {}
            LiteralNumber::Unread => {
                partial NUMBER_UNREAD => languages::Missing::Any;
            }
            LiteralNumber::Known { kind, value } => {
                row {
                    id: node(at),
                    kind: value(languages::intern_derived(symbols, tree.source, kind)),
                    value: value(languages::intern_derived(symbols, tree.source, value)),
                    file: file,
                    span: span(at),
                }
            }
        }
    }
}
