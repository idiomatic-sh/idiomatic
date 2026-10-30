//! The `nodes` service: free functions named by verb over the domain's types, with no IO and no ports. It says which of the
//! standard's categories a node is in.

use super::types::{Category, Node};

/// The standard's category of `node`: a rule, a declaration, a component value, or the stylesheet.
#[must_use]
pub(crate) const fn category(node: Node) -> Category {
    match node {
        Node::Stylesheet => Category::Stylesheet,
        Node::QualifiedRule | Node::AtRule => Category::Rule,
        Node::Declaration => Category::Declaration,
        Node::Function | Node::SimpleBlock => Category::ComponentValue,
    }
}
