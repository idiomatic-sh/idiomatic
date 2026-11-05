//! The `nodes` service: free functions named by verb over the domain's types, with no IO and no ports. It says which of the
//! standard's categories a node is in.

use super::types::{Category, CssNode};

/// The standard's category of `node`: a rule, a declaration, a component value, or the stylesheet.
#[must_use]
pub(crate) const fn category(node: CssNode) -> Category {
    match node {
        CssNode::Stylesheet => Category::Stylesheet,
        CssNode::QualifiedRule | CssNode::AtRule => Category::Rule,
        CssNode::Declaration => Category::Declaration,
        CssNode::Function | CssNode::SimpleBlock => Category::ComponentValue,
    }
}
