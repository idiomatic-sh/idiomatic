//! The `assigns` relation of the language: the string whose value is known that `=`, `:=`, `var` or `const` binds to a name, linked to
//! the name and the string, owned by the function it is in or the file's start. Each name is paired with the value at its place.

use crate::domain::definitions::service::assignments;
use crate::domain::nodes::types::GoNode;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `assigns` relation to, which reads its rows from a file's tree.
pub(crate) struct AssignsItem;

ryo::reads! {
    AssignsItem => languages::Assigns |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::AssignStmt), Kind::Node(GoNode::ValueSpec)] as kind_bits,
        each (assignments(tree, at)) as assigned {
            row {
                occurrence: node(at),
                binding: node(assigned.binding),
                value: node(assigned.value),
                owner: value(owner(tree, at)),
                file: file,
                span: span(assigned.value),
            }
        }
    }
}
