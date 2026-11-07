//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `oxc_parser` syntax adapter: the parser library the language reads with.
#[must_use]
pub(crate) fn oxc_parser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::oxc_parser::adapter::SyntaxOxcParserAdapter
}
