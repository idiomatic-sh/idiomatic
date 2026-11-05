//! The names of the `implementations` concept: the parts of an `impl`'s header its name leaves out.

use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::types::Kind;

/// The parts of an `impl` its name leaves out: its attributes, its generic parameters, the generic arguments of the types it names, and
/// its where clause.
pub(crate) const OMITTED: [Kind; 4] = [
    Kind::Node(RustNode::Attr),
    Kind::Node(RustNode::GenericParamList),
    Kind::Node(RustNode::GenericArgList),
    Kind::Node(RustNode::WhereClause),
];
