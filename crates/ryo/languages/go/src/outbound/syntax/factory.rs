//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `tree_sitter_go` syntax adapter: the parser library the language reads with.
#[must_use]
pub(crate) fn tree_sitter_go() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::tree_sitter_go::adapter::SyntaxTreeSitterGoAdapter
}
