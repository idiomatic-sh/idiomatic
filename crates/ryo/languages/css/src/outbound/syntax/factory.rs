//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `cssparser` syntax adapter: Servo's CSS parser.
#[must_use]
pub(crate) fn cssparser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::cssparser::adapter::SyntaxCssparserAdapter
}
