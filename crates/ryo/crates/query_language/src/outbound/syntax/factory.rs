//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The winnow syntax adapter, from what the edge passes.
#[must_use]
pub fn winnow() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::winnow::adapter::SyntaxWinnowAdapter
}
