//! The `assigns` relation of the language: the string whose value is known that an assignment binds to a single name, linked to the
//! name and the string, owned by the function it is in or the file's start.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::assignments;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::function_of;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `assigns` rows of one Python file from its syntax tree.
pub(crate) struct AssignsItem;

ryo::reads! {
    AssignsItem => languages::Assigns, model(Declarations) |tree, model, at, _symbols| {
        kinds: [Kind::Node(Node::Assign), Kind::Node(Node::AnnAssign), Kind::Node(Node::NamedExpr)] as kind_bits,
        each (assignments(tree, at)) as assigned {
            row {
                occurrence: node(at),
                binding: node(assigned.binding),
                value: node(assigned.value),
                owner: node(function_of(tree, at)),
                file: file,
                span: span(assigned.value),
            }
        }
    }
}
