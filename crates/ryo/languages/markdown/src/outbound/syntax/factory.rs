//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `pulldown_cmark` syntax adapter: the `CommonMark` pull parser.
#[must_use]
pub(crate) fn pulldown_cmark() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::pulldown_cmark::adapter::SyntaxPulldownCmarkAdapter
}
