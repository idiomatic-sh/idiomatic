//! The `inherits` relation of the language: each positional base of a class, by the dotted name it writes (`Generic[T]` names
//! `Generic`); a starred base has the unknown value of a star base, any other the unknown value of a dynamic base. A keyword such as
//! `metaclass=` is no base.

use crate::domain::extras::types::Extra;
use crate::domain::model::types::Declarations;
use crate::domain::names::service::base_target;
use crate::domain::nodes::types::Category;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, nodes_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `inherits` rows of one Python file from its syntax tree.
pub(crate) struct InheritsItem;

ryo::reads! {
    InheritsItem => languages::Inherits, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::ClassDef)] as kind_bits,
        given arguments = child(tree, at, Kind::Extra(Extra::CallArguments))?;
        each (nodes_of(tree, arguments, Category::Expression)) as base {
            row {
                occurrence: node(base),
                declaration: node(at),
                target: value(base_target(tree, symbols, base)),
                file: file,
                span: span(base),
            }
        }
    }
}
